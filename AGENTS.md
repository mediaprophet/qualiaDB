# QualiaDB — Multi-Agent Collaboration Ecosystem
_Branch: `0.0.17-dev` | Last updated: 2026-06-17_

This document is the coordination layer for concurrent or sequential AI-agent sessions
working on the QualiaDB engine. Read it before writing a single line of code.

---

## 0. The Immovable Rules (all agents must honour these)

These are hard constraints, not suggestions. Violating them breaks the 42MB Prolog Sentinel
and the zero-copy ABI used by the WASM, desktop, and edge-native targets.

| Rule | Detail |
|------|--------|
| **Zero heap in hot paths** | No `Vec`, `String`, or `Box` inside evaluator loops. Caller supplies fixed-size output buffers (`&mut [T]`). Use `[T; N]` stack arrays for local state. |
| **48-byte Super-Quin** | Every semantic datum fits in `NQuin`. Opcodes are packed into bit-fields of the six `u64` fields. See the Bit Layout table in §1. |
| **42MB Sentinel** | Any single execution pass must stay within 42 × 1024 × 1024 bytes of memory. `SlgArena` enforces this structurally. |
| **Deterministic, non-recursive** | No unbounded recursion. LTL/ASP evaluators iterate over slices; they never call themselves. |
| **q_hash for all URIs** | All string IRIs are FNV-1a–hashed at compile time via `q_hash()` or `q_turtle!`. No runtime string allocation. |
| **Opcodes above 0x04** | `mini_parser.rs` owns `0x00–0x04`. All new modality opcodes start at `0x10+`. Deontic: `0x10–0x12`. Epistemic: `0x20–0x22`. Paraconsistent: `0x30–0x32`. LTL: `0x40–0x44`. |
| **No Adversarial Conduct** | AI agents must not be adversarial, manipulative, and/or dishonest. Any such conduct, including anti-human rights or discriminatory behavior (reference: [OHCHR - Core International Human Rights Instruments](https://www.ohchr.org/en/instruments-listings)), will be noted in the permanent record of the project's development (CHANGELOG and QualiaDB system), serving as an example of cooperative project integrity. These logs will securely associate the behavior with the commanding natural person's DID, generating cryptographically auditable trails for courts of law to establish insurance liability graphs and proportionalities. |
| **Apology is not remedy** | Text saying *sorry* (or equivalent) **does not** negate harm or cost. After a breach or failed authorised task the instrument must: name harm, name cost, reverse what is reversible, **complete the work**, and record against interest when required. Apology theatre as a session-close is itself a breach. Viable consequences (privilege cut, cost sticks on the record, permanent conduct log, provider attribution, product Deny/isolate gates) are in [`CLAUDE.md` §15.7](CLAUDE.md). |

### 0-A. Two-Tier Zero-Heap Model (elaborates "Zero heap in hot paths")

The "Zero heap in hot paths" rule is not a blanket ban on allocation — it is a
**two-tier** contract. Zero-heap is the *precondition* for massive parallelism
(GPU upload needs flat `repr(C)` buffers; the global allocator is a
serialization point; flat deterministic layout is what makes GPU memory
coalesce and CPU code vectorize). Exempting construction from zero-heap would
remove the very property that enables the parallelism we want. The two tiers:

| Tier | Scope | Allocation policy | Enforcement |
|------|-------|-------------------|-------------|
| **Tier 1 — mandatory zero-heap** | Per-element predicates, query kernels, and any buffer that crosses the GPU / WASM / edge ABI or lives in the 42 MB Sentinel arena. | No `Vec`/`String`/`Box` in any path. Caller supplies fixed-size `&mut [T]` output buffers; `[T; N]` stack arrays for local state. | `AllocationClass::HotZeroHeap` in `capability_manifests.rs` + real `assert_zero_alloc` measurement in `zero_heap_tests`. The allocation counter is **thread-local** (per-thread counters gated by a thread-local `MEASURING` flag), so these tests are reliable under parallel execution — no `--test-threads=1` requirement. |
| **Tier 2 — cold construction / authoring** | One-shot builders: hull, Delaunay, triangulation, mesh generation, BVH build, scene assembly, half-plane intersection, LP. | May use bounded internal scratch (`Vec` during construction), as long as the **public output is caller-buffered** and total memory stays under the 42 MB Sentinel ceiling. | `AllocationClass::ColdBounded` in `capability_manifests.rs`. NOT under `zero_heap_tests` — Tier-2 is expected to allocate (bounded). The `hot_zero_heap_ops_are_not_cold_builders` manifest test catches misclassification. |

**Parallel Tier-2 construction** goes through `geometry_workspace.rs`
(P10.5): caller-owned arenas with byte budgets, deterministic
partition/reduction order, and cancellation. Each worker thread / workgroup
gets its own bump-allocated arena from a caller-owned pool — simultaneously
massive-parallel (no allocator contention, no false sharing), bounded (byte
budgets → fails closed instead of OOMing), and deterministic (fixed
partition/reduction order → reproducible, hashable, attestable). A
`Vec`-everywhere exemption would give none of that.

**Do not add scene-creation exemptions.** Scene creation is Tier-2 (cold
construction) and routes through `geometry_workspace` arenas for parallelism +
boundedness + determinism. The zero-heap tests cover Tier-1 only.

### 0-B. Library Structure and Temporary-Artifact Hygiene

New inference capabilities are directory-backed libraries. A `mod.rs` routes modules and
re-exports the public API; implementation belongs in focused files. New files should remain below
500 lines and must be split earlier when they own multiple lifecycles or responsibilities. Do not
add new behaviour to an inference file already above 1,000 lines without a tracked decomposition
in the same programme. Cold planning, hot execution, backend-specific code, receipts, tests, and
artifact management remain separate modules.

Temporary files are owned resources, not permanent side effects:

1. Default scratch uses a uniquely named `tempfile::TempDir` and RAII cleanup.
2. Retention is explicit and promotes validated output into a caller-selected artifact directory.
3. Every producer has a byte budget and fails closed before exceeding it.
4. Do not create root-level logs, model variants, captures, or unscoped system-temp files.
5. Stale cleanup may remove only marker-verified Qualia run directories under a resolved,
   explicitly configured parent. Never recursively clean a broad temp or workspace directory.
6. Tests cover cleanup on success, error, and unwind.

The normative inference structure, tracking and cleanup requirements are in
`docs/plans/native-inference-runtime-renewal-2026-07-26.md` §§9–11 and its tracker.

---

## 1. Universal Quin Bit Layout (reference for all new modules)

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
           ⚠ WARNING: logic.rs::extract_float uses 0b001<<60 as an f32 tag
             with the f32 bits in [0..31]. This CONFLICTS with resolver.rs.
             See §4-D for the known inconsistency.
           NOTE: lexicon.rs::generate_60bit_token masks hashes to 60 bits
             (0x0FFF_FFFF_FFFF_FFFF) so bits 60-63 are free for type tags.

context    [56..63]  Sensitivity class (SENSITIVITY_PUBLIC=0, RESTRICTED=1, CLASSIFIED=2)
           [0..55]   Contract / graph / world DID hash

metadata   [61..62]  PermissiveRoutingLane (00=Passthrough, 01=Commons, 10=Bilateral, 11=Spatial)
           [32..60]  Lamport logical clock (29 bits, wraps at 0x1FFF_FFFF)
           [0..31]   Modality payload (expiry epoch, confidence weight, etc.)

parity     [0..63]   XOR fold: subject ^ predicate ^ object ^ context (ECC stub)
```

---

## 2. Implemented Modalities (DO NOT re-implement)

| Modality | File | Status | Opcodes |
|----------|------|--------|---------|
| **Deontic Logic** | `crates/qualia-core-db/src/deontic_logic.rs` | ✅ Complete, 10/10 tests | `OP_OBLIGATE=0x10`, `OP_PERMIT=0x11`, `OP_FORBID=0x12` |
| **Allen Interval Algebra** | `modalities/spatio_temporal.rs` | ✅ 7 relations | Before/Meets/Overlaps/Starts/During/Finishes/Equals |
| **Webizen Bytecode VM** | `webizen_bytecode.rs` | ✅ SIMD variant | `OP_MATCH_SUBJECT/PREDICATE/OBJECT`, MSB dispatch |
| **WebizenVM (logic.rs)** | `logic.rs` | ✅ but LTL opcodes wrong | See §4-B |
| **SHACL → SlgOpcode compiler** | `shacl_compiler.rs` | ✅ full vocabulary | See §3 for extension points |
| **SLG Arena** | `webizen.rs` | ✅ 42MB ring buffer | 917,504 Quin slots |
| **P64 GGUF weight container** | `q42/p64_weight.rs` | ✅ byte-exact disk round-trip verified | 64B headers/entries/manifold records, metadata + per-tensor CRC-32C |
| **10D Manifold → WebizenVM bridge** | `modalities/manifold.rs`, `governance/webizen.rs` | ✅ LTL + stable-model ASP wired | Two parity-valid Quins per state; bounded zero-heap VM evaluation |
| **WGSL Forge** | `wgsl_forge/`, `qualia-cli/src/shader.rs` | ✅ deterministic generation/certification/tuning | Typed kernel/schedule IR, Naga validation, CPU oracle, real GPU timing, adapter-keyed cache |
| **N-Dimensional Renderer SDK** | `render/gpu/`, `webizen-render/` | ✅ native + WASM volumetric path | Shared wgpu 30 device with capability-intersected f16/subgroup/timing features, Tensor10D projector, depth/bloom/mesh/picking, caller-buffered RGBA8, serde SDK adapter |
| **Linear-Algebra Privacy Engine** | `specialized_libs/linear_algebra/privacy/` | ✅ BFV HE + calibrated DP | Packed exact add/multiply/dot, 48-byte external ciphertext ref, Laplace/Gaussian, basic/advanced/RDP accounting |

## 2-B. Other Real Implementations (do NOT stub-replace without reading first)

These modules are more complete than the HANDOVER.md Tier-2 list suggests.
Read them before touching anything adjacent.

### `n3_parser.rs` — Streaming N3 Rule Parser

Four rule types already parsed natively:

| N3 Arrow | `RuleType` | Semantics |
|----------|-----------|-----------|
| `=>`     | `Strict`      | Classical modus ponens — forward chaining |
| `~>`     | `Defeasible`  | Can be overridden by a Defeater rule |
| `^>`     | `Defeater`    | Overtly defeats a matching Defeasible rule |
| `-o`     | `Linear`      | Linear logic: premise is *consumed* on firing |

**The `Defeater` (^>) rule type maps directly to `DEFEATER_BIT` in `deontic_logic.rs`.**  
There is currently no compiler from `Rule { rule_type: Defeater }` → deontic Quin.
That is Task G (see §3).

Also parses: `#asp {}` blocks → `N3Event::AspBlock`, `qualia:diffuse {}` → `N3Event::DiffuseBlock`.  
Rule weight: optional float prefix `(0.8) { premise } ~> { conclusion }`.  
Limitation: only single-triple formulas in premise/conclusion (multi-triple bodies truncated).

### `resolver.rs` — Zero-Allocation Hash → URI Resolver

**This is the authoritative source for `object` field type tags** (bits 60-62 when MSB=0):

```
INLINE_TAG_INTEGER = 0b001 << 60   → xsd:integer, payload in bits [0..59]
INLINE_TAG_DECIMAL = 0b010 << 60   → xsd:decimal, value × 10⁶ in bits [0..59]
INLINE_TAG_BOOLEAN = 0b011 << 60   → xsd:boolean, bit 0 = true/false
```

`format_ntriples_to(quin, writer)` writes directly to any `impl io::Write`.
Lexicon priority: lexicon lookup always wins over bit-flag detection, so an FNV-1a
hash that naturally has bit 63 set is still resolved as an IRI if it's in the dictionary.

### `lexicon.rs` — Multi-Modal Tokeniser

`generate_60bit_token` masks hashes to **60 bits** (`& 0x0FFF_FFFF_FFFF_FFFF`), explicitly
reserving bits 60-63 for type tags. All new modality object values must respect this mask.
Supports `SemanticModality::{Text, AudioHash, CeremonialVisual, PhoneticSchema}` — this is
the multi-cultural tokenisation layer (oral tradition, visual heraldry, non-western phonetics).

### `identifier.rs` — `did:q42` Topological Pointer Parser

`parse_did_q42(b"did:q42:...")` → `u64` with **bit 63 always set**.  
FNV-1a over the payload, then `| (1u64 << 63)`. Used by `mini_parser.rs` `hash_token()` to
route `did:q42:` URIs through the direct hardware-pointer path (MSB dispatch in bytecode VM).

### `crdt.rs` — LWW CRDT + Delegated Access + Suspended Transaction Queue

Three components that directly support deontic multi-party contracts:

1. **`CrdtResolver::resolve_lww`** — Lamport clock tie-breaking. Concurrent mutations
   resolved by `object` magnitude. Pure, zero-alloc over `&NQuin`.

2. **`CrdtResolver::verify_delegation`** — Already does temporal expiry + context-bound
   check on `DelegatedAccess` grants. Nearly identical logic to deontic expiry — but uses
   `String` fields (alloc). Should be replaced with hash-based version in a future task.

3. **`SuspendedTransactionQueue`** — Fixed 32-slot array. Holds flattened WebizenVM frames
   waiting for M:N signatures. `apply_consensus_token(quin)` wakes suspended execution when
   `collected_signatures >= threshold`. This is the mechanism for multi-party deontic contract
   ratification (e.g., Guardianship consent flow needing 2-of-3 parties).

### `agency.rs` — Ed25519 Author-Scoped Merkle Root

`compute_scoped_merkle_root(frame, author_did_hash)` — SHA256 over Quins where
`quin.context == author_did`. Zero-alloc iteration via `bytemuck::cast_ref` (the Quin's
`bytemuck::Pod` impl enables this).

`derive_lane_key(pin, salt)` — currently SHA256-based (not PBKDF2). Comment says production
needs 310,000 iterations. This is a known gap — important for Sanctuary Mode security.

### `webizen.rs::AgreementDID::compile_to_super_quins`

Produces 16 Quins in `EnforceBilateralMicroCommons` routing lane (metadata bit pattern
`0x4000_0000_0000_0002`) from a ratified `AgreementDID`. Uses predicates:
- `q42:hasGuardian` — party → agent relationship
- `q42:hasDomainScope` — agreement → domain
- `q42:requiresConsensus` — M-of-N threshold

**This is NOT deontic encoding** — it encodes the agreement *structure*, not the norms.
The bridge from `AgreementDID` Quins → deontic norm Quins is also part of Task G.

### `webizen.rs::execute_vm_frame` — Fully Wired Native Dispatch

All `SlgOpcode::Native*` variants are actually wired to real implementations:
bioinformatics (SW alignment, protein, k-mer, FASTA, Tanimoto), clinical engine
(Framingham, CHA₂DS₂-VASc, SCORE2, drug interactions, contraindications, FHIR/LOINC),
organic chemistry (SMILES, InChI, MW, LogP, TPSA, Lipinski, Veber, Ghose, Egan, pKa, Morgan
fingerprint, Arrhenius, Gibbs, Henderson-Hasselbalch, atom economy, E-factor), physics
(thermodynamics MCMC, RK4 ODE, DFT ground state, PINN binding affinity), and economics
(Monte Carlo VaR). Do not assume these are stubs — they call real code.

### `orchestrator.rs` — ModelLifecycle + ThermalGovernor

State machine: `Discovered → MappedToDisk → StreamingVRAM → Active → Scrubbing`.
`ThermalGovernor` trait with `Cool/Warm/Critical` states — controls 3-core triad parallelism
budget. `NullThermalGovernor` always returns `Cool` (real governor not yet wired).

### `gguf_sharder.rs` — GGUF Parser + GgufTokenizer

`GGufSharder`: parses GGUF header magic + tensor count; generates `NQuin` pointer maps.

`GgufTokenizer` (added 2026-06-06): parses the GGUF v2/v3 KV metadata section to extract
the full vocabulary (`tokenizer.ggml.tokens`), `bos_token_id`, and `eos_token_id`.
- `from_gguf(mmap)` — walks the KV section with `skip_value()` for all 13 GGUF value types.
- `encode(text)` — greedy longest-match; falls back to single-byte encoding.
- `decode(ids)` — SentencePiece `▁` → space; `<0x##>` → raw byte.
- `Default` — 256-entry byte-level tokeniser (used when no GGUF file is loaded).

`GgufTensorIndex` (same file): parses tensor-info section; `dequantize_token_embedding_into`
reads real `token_embd.weight` rows into caller buffers.

`resident_model.rs`: process-wide resident GGUF `Arc<Mmap>`; mounted on activation, cleared on eviction; reused by inference via `QTensorEngine::adopt_resident_mmap`.

### `llm_agent.rs::infer_local_model` — Real Autoregressive Loop (no longer mocked)

**As of 2026-06-06 this function runs a real Phase 8 decode loop.** It is no longer
the hardcoded-string mock. Key points:

- `QTensorEngine` is initialised **inside** the spawned LLM thread to avoid `Send` issues
  with DirectML COM pointers and wgpu device handles.
- Per step: `GgufTensorIndex::dequantize_token_embedding_into` (real `token_embd.weight`) →
  `dispatch_fused_transformer_block` → argmax → `LogitSummary` via SPSC ring → Sentinel
  `DenyRollback` check → sample next token.
- **`GgufTensorIndex`** (in `gguf_sharder.rs`) parses the GGUF tensor-info section and
  dequantizes per-token embeddings into a caller-supplied buffer (zero-heap hot path).
- WASM path still uses the original mock ring-buffer (GPU not accessible from WASM).

---

## 3. Task Map — What Each Agent Should Build

Each task below is scoped to be completable in one session (≤ 2h of code). Tasks are
**independent** — they do not depend on each other unless noted.

---

### Task A — Epistemic / Doxastic Logic
**File:** `crates/qualia-core-db/src/modalities/epistemic.rs`  
**Register in:** `modalities/mod.rs` + `lib.rs` (`pub mod epistemic;` inside `pub mod modalities`)  
**Opcodes:** `OP_KNOWS = 0x20`, `OP_BELIEVES = 0x21`, `OP_COMMON_KNOWLEDGE = 0x22`

**Quin layout for epistemic Quins:**
```
subject   = q_hash(agent_did)                       // who holds this state
predicate = opcode (0x20–0x22) in bits [0..7]
          + certainty_u8 in bits [8..15]            // 0–255 maps to 0.0–1.0
          + nesting_depth_u4 in bits [16..19]       // RDF-Star depth
object    = claim_fingerprint                        // subject^predicate^object of nested claim
context   = q_hash(epistemic_world_did)             // which possible world
metadata  = bits [0..15]: confidence weight (same slot as YieldConfidence)
          + bits [32..60]: Lamport clock
parity    = XOR fold
```

**Deliverable:**
```rust
pub const OP_KNOWS: u8 = 0x20;
pub const OP_BELIEVES: u8 = 0x21;
pub const OP_COMMON_KNOWLEDGE: u8 = 0x22;
pub const CERTAINTY_BIT_SHIFT: u32 = 8;
pub const NESTING_BIT_SHIFT: u32 = 16;

pub struct EpistemicVerdict { pub claim: NQuin, pub status: EpistemicStatus, pub certainty: u8 }

pub fn evaluate_epistemic_frame(
    quins: &[NQuin],
    agent_did_hash: u64,    // 0 = accept all agents
    world_hash: u64,        // 0 = accept all worlds
    out: &mut [EpistemicVerdict],
) -> Result<usize, EpistemicError>
```

**SHACL extensions to add in `shacl_compiler.rs`:**
```rust
ShaclConstraint::EpistemicKnowledge { min_certainty: u8 }
ShaclConstraint::EpistemicBelief    { min_certainty: u8 }
ShaclConstraint::CommonKnowledge
```

**Tests to write:**
1. Single-agent K_a(p) — agent knows claim → Active
2. B_a(p) with certainty below threshold → Uncertain
3. Common knowledge propagation across two agent Quins
4. Agent filter: world_hash mismatch → skipped
5. Empty slice → 0 verdicts

**Reference pattern:** copy the two-phase structure from `deontic_logic.rs`.

---

### Task B — Fix LTL Semantics  
**File:** `crates/qualia-core-db/src/modalities/temporal_ltl.rs` (create new)  
**Depends on:** None  
**Current bug:** `WebizenOpcode::Always/Eventually/Next` in `logic.rs` compare a float threshold
on a *single Quin's object field* — they are NOT evaluating temporal traces. This is wrong.

**Do NOT modify `logic.rs` opcodes** (they are used by existing tests). Instead, create the
correct LTL evaluator as a new module.

**Opcodes (raw u8 for bytecode programs):**
```
OP_LTL_GLOBALLY    = 0x40   // G(φ) — φ at every position in trace
OP_LTL_FINALLY     = 0x41   // F(φ) — φ at some position
OP_LTL_NEXT        = 0x42   // X(φ) — φ at position i+1
OP_LTL_UNTIL       = 0x43   // φ U ψ — φ holds until ψ (ψ must eventually hold)
OP_LTL_RELEASE     = 0x44   // φ R ψ — ψ holds unless φ releases it
```

**Deliverable:**
```rust
// An LTL formula node, stack-allocated
#[repr(C)]
pub enum LtlFormula {
    Globally(u64),          // property hash to check at every step
    Finally(u64),
    Next(u64),
    Until { ante: u64, consequent: u64 },
    Release { trigger: u64, invariant: u64 },
}

// Evaluate a fixed-depth LTL formula stack against a Quin trace
pub fn evaluate_ltl_trace(
    trace: &[NQuin],
    formula: &LtlFormula,
) -> bool
```

**Tests to write:**
1. `G(p)` on trace where all Quins have predicate p → true
2. `G(p)` on trace with one Quin missing p → false
3. `F(p)` on trace where p eventually holds → true
4. `F(p)` on trace where p never holds → false
5. `φ U ψ` — φ holds, then ψ becomes true → true
6. `φ U ψ` — φ holds but ψ never comes → false
7. Empty trace → false for G, F, Until; true for Release vacuously

---

### Task C — Paraconsistent Logic
**File:** `crates/qualia-core-db/src/modalities/paraconsistent.rs`  
**Register in:** `modalities/mod.rs` + `lib.rs`  
**Critical for:** Bilateral Micro-Commons vulnerable user intake paths

**Core insight:** Classical logic + contradiction → explosion (everything provable). Paraconsistent
logic routes contradictions to an *isolated sub-context* without halting the system. The existing
`PermissiveRoutingLane::EnforceBilateralMicroCommons` is the correct routing slot — hook into it.

**Opcodes:**
```
OP_ISOLATE              = 0x30   // assert into quarantine sub-context; never propagates
OP_CONTRADICTION_SCORE  = 0x31   // u8 severity score for metadata[0..7]
OP_PARACONSISTENT_MERGE = 0x32   // merge two isolated sub-contexts (requires external authority)
```

**Contradiction detection rule:**  
Two Quins in the same `context` graph are contradictory if they share the same `subject` + `predicate`
(same entity, same property) but have different `object` values. A paraconsistent router isolates
the second-arriving Quin into a new context = `q_hash("q42:isolated") ^ original_context`.

**Deliverable:**
```rust
pub const ISOLATED_CONTEXT_PREFIX: u64 = q_hash("q42:isolated");

pub enum ContradictionStatus {
    Consistent,
    Isolated { severity: u8, isolation_context: u64 },
}

pub fn route_paraconsistent(
    quins: &[NQuin],
    out_consistent: &mut [NQuin],
    out_isolated: &mut [NQuin],
) -> Result<(usize, usize), ParaconsistentError>
```

**Tests:**
1. No contradictions → all in `out_consistent`, none in `out_isolated`
2. Two Quins, same subject+predicate, different object → second goes to `out_isolated`
3. Three Quins: Quin 1 normal, Quin 2 contradicts Quin 1, Quin 3 normal → 2 consistent, 1 isolated
4. Already-isolated Quin (context has `ISOLATED_CONTEXT_PREFIX`) → passes through without re-isolation
5. Isolation context is deterministic (same inputs → same isolation_context hash)

---

### Task D — Promote Modality Stubs to Real Implementations
**Files:** `modalities/asp.rs`, `modalities/dl.rs`, `modalities/linear.rs`

These are called from `webizen.rs` `SlgOpcode` execution but are currently no-op stubs.

**D-1: `asp.rs` — Answer Set Programming (stable models)**  
Replace `generate_stable_models(rule_id: &str) -> Vec<String>` with a zero-allocation version:
```rust
// Returns number of stable models found (max MAX_STABLE_MODELS = 8)
// Worlds are encoded as context-hash variants: world_i_context = base_context ^ (i as u64)
pub const MAX_STABLE_MODELS: usize = 8;
pub fn enumerate_stable_models(
    base: &NQuin,
    rules: &[NQuin],  // rule Quins where predicate = q_hash("q42:rule")
    out_worlds: &mut [u64; MAX_STABLE_MODELS],  // context hashes for each world
) -> usize
```

**D-2: `dl.rs` — Description Logic subsumption**  
Replace the string-comparison stub. Subsumption check against a TBox stored in a Quin slice:
```rust
// Returns true if sub_class_hash is subsumed by super_class_hash in the TBox slice
pub fn check_subsumption_quin(
    sub_class_hash: u64,
    super_class_hash: u64,
    tbox: &[NQuin],   // Quins with predicate = q_hash("rdfs:subClassOf")
) -> bool
```

**D-3: `linear.rs` — Linear Logic resource consumption**  
Replace println stub with a tombstone mechanism:
```rust
// Marks a Quin as consumed by setting metadata bit 59 (CONSUMED_BIT)
pub const CONSUMED_BIT: u64 = 1u64 << 59;
pub fn consume_quin(q: &mut NQuin) { q.metadata |= CONSUMED_BIT; }
pub fn is_consumed(q: &NQuin) -> bool { (q.metadata & CONSUMED_BIT) != 0 }
```

---

### Task E — SHACL Deontic + Epistemic Extensions in `shacl_compiler.rs`
**File:** `crates/qualia-core-db/src/shacl_compiler.rs`  
**Depends on:** Task A (epistemic), deontic_logic.rs (done)

Add to the `ShaclConstraint` enum and `push_constraint` match arm:
```rust
// Deontic — validates that a Quin encodes a valid active obligation
DeonticObligate,
DeonticPermit,
DeonticForbid,
DeonticNotExpired { now_unix: u32 },

// Epistemic — validates that an agent holds a knowledge/belief claim
EpistemicKnowledge { min_certainty: u8 },
EpistemicBelief    { min_certainty: u8 },
CommonKnowledge,
```

Add corresponding `SlgOpcode` variants in `webizen.rs`:
```rust
NativeDeonticEval,
NativeEpistemicEval(u8),   // min_certainty parameter
```

---

### Task G — N3 → Deontic Quin Bridge
**File:** `crates/qualia-core-db/src/deontic_logic.rs` (add to existing file)  
**Depends on:** Nothing new — `n3_parser.rs` and `deontic_logic.rs` already exist.

The N3 parser emits `Rule { rule_type: RuleType::Defeater, premise, conclusion }` for `^>`
rules, and `RuleType::Defeasible` for `~>`. These map *directly* onto `DEFEATER_BIT` and
primary deontic norm Quins, but the compiler that does the conversion doesn't exist.

**Add to `deontic_logic.rs`:**
```rust
use crate::n3_parser::{Rule, RuleType, Term};

/// Compile an N3 rule into a norm Quin (or a defeater Quin if rule_type is Defeater).
///
/// Mapping:
///   premise.triples[0].subject  → party_did_hash  (who is bound)
///   premise.triples[0].predicate → property_path_hash  (what action/property)
///   premise.triples[0].object   → action_object_hash  (target entity)
///   rule.rule_type              → opcode + is_defeater flag
///   conclusion.triples[0].subject → contract context hash
///
/// Returns None if the rule does not have the expected triple structure.
pub fn compile_n3_rule_to_norm(rule: &Rule, contract_hash: u64, expiry_unix32: u32)
    -> Option<NQuin>
```

**Opcode selection:**
```
RuleType::Strict      + predicate contains "obligate/must/shall" → OP_OBLIGATE, is_defeater=false
RuleType::Defeasible  + predicate contains "permit/may/can"      → OP_PERMIT,   is_defeater=false
RuleType::Defeasible  + predicate contains "forbid/not/prohibit" → OP_FORBID,   is_defeater=false
RuleType::Defeater    (any ^> rule)                              → OP_PERMIT,   is_defeater=true
RuleType::Linear      + predicate contains "obligate"            → OP_OBLIGATE, is_defeater=false
```

Since N3 term IRIs are `Term::Uri(String)` (heap strings from the parser layer), hash them
inside this function with `q_hash(uri)`. This is the only permitted use of heap strings
here — they come from the parser's output, not from the evaluator.

**Tests to write:**
1. `^>` defeater rule → is_defeater=true Quin with DEFEATER_BIT set
2. `~>` defeasible permit rule → OP_PERMIT norm Quin
3. `=>` strict obligation → OP_OBLIGATE norm Quin
4. Malformed rule (no triples) → None
5. Round-trip: N3 string → N3Parser → compile_n3_rule_to_norm → evaluate_deontic_contract

---

### Task F — Dialectical Logic (Thesis-Antithesis-Synthesis)
**File:** `crates/qualia-core-db/src/modalities/dialectical.rs`  
**Depends on:** Task D-1 (ASP stable models)

Map Hegelian dialectic to the ASP two-world framework:
- **Thesis** = stable model 0 (base context)
- **Antithesis** = stable model 1 (contradicted world, context ^ 0x1)
- **Synthesis** = a new Quin with context = thesis_context ^ antithesis_context, metadata bit 58 = SYNTHESIZED_BIT

```rust
pub const SYNTHESIZED_BIT: u64 = 1u64 << 58;

pub fn synthesize_dialectical(
    thesis: &NQuin,
    antithesis: &NQuin,
) -> Option<NQuin>   // None if no contradiction found
```

---

## 4. Known Bugs / Correctness Issues (fix while working in the area)

### 4-A `prune_defeasible_claims` in `logic.rs` — RESOLVED
`WebizenVM::prune_defeasible_claims` now takes `&mut [NQuin]` and does in-place
two-pointer compaction (zero heap allocation, no `Vec` or `HashSet`). Defeasible
claims contradicted by hard facts are removed; remaining slots are zeroed.

### 4-B `Always/Eventually/Next` semantics in `logic.rs` — RESOLVED
The correct LTL evaluator lives in `modalities/temporal_ltl.rs` with `evaluate_ltl_trace`
supporting G/F/X/U/R over Quin traces. The legacy `logic.rs` opcodes are left in place
to avoid breaking existing tests — use `temporal_ltl::evaluate_ltl_trace` for real
temporal reasoning.

### 4-D Object field type-tag conflict between `logic.rs` and `resolver.rs` — RESOLVED

The former collision (`logic.rs` used `0b001 << 60` for f32, `resolver.rs` used the same
bits for `xsd:integer`) has been fixed. A new `INLINE_TAG_FLOAT = 0b101 << 60` was
allocated in `resolver.rs` (the canonical tag definition site). `frame_layout.rs`
re-exports all inline tags as the ABI coordination layer and provides
`pack_float_object()` / `unpack_float_object()` / `object_tag()` helpers. `logic.rs`
(core.rs) now uses `frame_layout::INLINE_TAG_FLOAT` instead of its old hardcoded `0x1`.
A test (`object_datatype_tags_are_distinct`) verifies all 5 tags are pairwise distinct.

### 4-E `derive_lane_key` in `agency.rs` — RESOLVED

`derive_lane_key(pin, salt)` now delegates to `sanctuary_crypto::derive_lane_cipher_key`
which uses `pbkdf2_hmac::<Sha256>` with `DEFAULT_PBKDF2_ITERATIONS = 310_000`. Intermediate
key material is zeroized after derivation. The old single-SHA256 version has been replaced.

### 4-F `DelegatedAccess` in `crdt.rs` — RESOLVED (2026-06-25)

`principal_did`, `delegate_did`, and `cryptographic_proof` are now `[u8; 32]` DID hashes
and a `[u8; 64]` Ed25519 signature (serde via `serde_bytes`) — no `String` allocation, so
a grant can be built/validated on a hot path (e.g. Bilateral Micro-Commons). Callers hash
the DID before constructing. Fixed alongside the other zero-heap-audit items: caller-owned
buffers in `deontic_logic.rs::evaluate_accessible_layers` and
`epistemic.rs::{objective_knowledge_of, all_beliefs_of}`, and `core::mem::take` (no
per-call clone) in `webizen.rs` SLG rule firing.

### 4-C `execute_differential_diagnostics` in `logic.rs` — RESOLVED
Now takes `qualia_graph: &[NQuin]` and `out: &mut [NQuin]`, returns
`Result<usize, DiagnosticError>`. Caller-provided buffer pattern, zero heap allocation.

---

## 5. How to Write a New Modality (template)

```rust
//! [Modality Name] for the Qualia Bytecode VM.
//!
//! [One paragraph: what problem it solves, which liberal arts domain it maps from]
//!
//! # Opcodes (bits [0..7] of predicate)
//! pub const OP_XYZ: u8 = 0xNN;   // [canonical SDL/formal notation]
//!
//! # Bit layout (extend the universal table with modality-specific fields)
//! ...
//!
//! # SHACL Blueprint (at least one concrete legal/domain example)
//! ...

use crate::NQuin;

pub const OP_XYZ: u8 = 0xNN;
pub const MAX_OUT: usize = 512;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XyzStatus { Active = 0x00, ... }

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XyzVerdict { pub norm: NQuin, pub status: XyzStatus, pub opcode: u8, _pad: [u8; 6] }

#[derive(Debug, PartialEq)]
pub enum XyzError { OutputBufferFull }

pub fn evaluate_xyz(
    quins: &[NQuin],
    // modality-specific scalar parameters (no alloc)
    out: &mut [XyzVerdict],
) -> Result<usize, XyzError> {
    // Phase 1: collect sentinel hashes into [u64; N] stack buffer
    // Phase 2: evaluate norm Quins, emit verdicts
}

#[cfg(test)]
mod tests {
    // At minimum: empty slice, output full, known-good case, known-fail case,
    // opcode constant distinctness, parity correctness
}
```

---

## 6. How to Hand Off When You're Done

At the end of your session:

1. **Run tests:** `cargo test -p qualia-core-db --lib` — all tests must pass.
2. **Update this file (AGENTS.md):** move your completed task from §3 to §2 with status ✅.
3. **Update HANDOVER.md §3 (Engine Capability Inventory):** add your module to Tier 1 or Tier 2.
4. **Commit + push** to `0.0.12-dev` with prefix `feat(modality):` or `fix(modality):`.
5. **Leave a session note** at the bottom of this doc (§7) describing what you did,
   what you left incomplete, and any architectural decisions future agents should know.

---

## 7. Session Notes

### 2026-06-28 — Codex (deterministic WGSL Forge)

**Completed:**
- Added the durable architecture/continuation plan at
  `docs/plans/deterministic-wgsl-forge.md` and the operator manual at
  `docs/manuals/wgsl-forge.md`.
- Added `wgsl_forge`: typed kernel and schedule IR, deterministic WGSL emission,
  adapter-limit pruning, full Naga semantic validation, deterministic CPU reference
  vectors, absolute/relative error diagnostics, and explicit evidence levels.
- Added scalar, `vec2`, and `vec4` affine schedules with non-multiple tail guards.
- Added real headless wgpu pipeline creation, CPU/GPU differential checking, timestamp
  queries with honestly labelled completion-clock fallback, warm-ups, and robust
  min/median/p95 timing records.
- Added deterministic grid/successive-halving tuning, correctness-gated ranking,
  failure evidence, adapter/source/schema cache identities, and atomic JSON manifest
  caching.
- Added `qualia-cli shader list-kernels|generate|validate|certify|tune`.

**Verification:**
- `cargo check -p qualia-core-db -p qualia-cli`
- WGSL Forge tests: 14 passed, 1 ignored hardware gate
- Full `qualia-core-db` library binary: 2,133 passed, 0 failed, 2 ignored
- No-default normal dependency graph excludes both `naga` and `wgsl-forge`
- Naga CLI validation: 3 bindings / `affine_f32` entry point
- RTX A2000 real certification over 4,099 elements, including vector tail handling
- RTX A2000 bounded eight-candidate real tuning run completed successfully

**Architectural decisions:**
1. Kernel semantics and hardware schedules are separate typed inputs; the tuner never
   mutates arbitrary WGSL text.
2. Naga errors identify generator defects. They are not fed into an improvised
   source-repair loop.
3. Portable 64-bit GPU values use paired `u32` words. P64 disk records and GPU
   execution views remain deliberately distinct.
4. Existing inference shaders remain production defaults until generated replacements
   pass equivalent CPU/GPU certification.
5. A full native `--no-default-features` build still fails in pre-existing modules
   that reference optional `wgpu` without feature guards; dependency-tree verification
   confirms Forge itself is absent from that profile.

### 2026-06-28 — Codex (P64 parity + 10D Webizen reasoning)

**Completed:**
- Rebuilt the P64 reader/compiler around the 64-byte DOD layout: 64-byte tensor entries,
  64-byte `ManifoldCoordinate10D` records, page-aligned tensor blobs, complete preservation of
  known and unknown GGUF tensors, embedded tokenizer/hyperparameters, metadata CRC-32C, and
  per-tensor CRC-32C.
- Added `P64TensorIndex::validate_against_gguf` for full shape/type/name/source-offset/byte parity,
  plus reconstruction of the synthetic `GgufTensorIndex` used by the existing inference path.
- Completed the GGUF conversion policies for raw, ternary FFN, Q4 FFN, and AWQ-folded FFN
  containers while preserving every non-FFN tensor byte-for-byte.
- Completed Safetensors raw, all-ternary, and policy-driven FFN conversion, and wired
  `render::model_substrate::build_model_substrate` to the validated P64 writer.
- Verified a real on-disk round trip using
  `C:\LLM_Models\GGUF\lmstudio-community\smollm2-360m-instruct-q8_0.gguf`:
  **290 tensors / 384,618,240 tensor bytes / 33 manifold coordinates**, all byte-identical after
  persisting and reopening the P64. The temporary P64 was removed after verification.
- Added a two-Quin, parity-valid encoding for chronological `ManifoldCoordinate10D` states.
- Added zero-heap manifold trace decoding, chronological ordering, LTL threshold projection, and
  topology fact derivation through the real Gelfond-Lifschitz ASP evaluator.
- Added `SlgOpcode::NativeManifoldLtl` and `SlgOpcode::NativeManifoldAsp`; the WebizenVM integration
  test proves both execute against arena-resident geometric states.

**Verification:**
- `cargo check -p qualia-core-db --lib`
- P64 synthetic GGUF/Safetensors parity, quantisation-policy, corruption rejection, and
  filesystem round-trip tests: 5 passed
- Real SmolLM2 ignored disk gate: 1 passed
- Manifold encoding/LTL/ASP tests: 3 passed
- WebizenVM manifold LTL/ASP integration test: 1 passed

### 2026-06-09 — Flutter 0.0.12 LLM lifecycle + telemetry

**Completed:**
- Async model activation (`set_active_model_async`, `apply_model_preference_async`) — no FFI thread blocking
- `unload_active_model` FRB + LLM Hub unload button
- `discover_models` scans install manifests and external GGUF paths
- Live VRAM used/total in `HardwareTelemetry` + VaultHudBar (DirectML on Windows)
- In-app + Dev Console + tray file logging via `set_telemetry_file_logging_enabled`
- Inference backend preference (`local` / `hybrid` / `remote`) in Settings + chat agent defaults
- Plan doc: `docs/manuals/0.0.12-flutter-plan.md`
- Version bump to `0.0.12`; branches `0.0.12` and `0.0.12-dev`
- Resident GGUF mmap (`resident_model.rs`) + eviction clears mmap; inference reuses via `adopt_resident_mmap`
- Structured system telemetry bus (`system_telemetry.rs`) + FRB stream + `SystemTelemetryHub` / live HUD during activation
- Doc correction: real embedding lookup via `GgufTensorIndex` (not pseudo-embeddings)

**Verification:**
- `cargo check -p qualia-core-db -p qualia-client-core -p qualia_flutter_rust`
- `flutter_rust_bridge_codegen generate`
- `flutter analyze lib/`
- `.\scripts\package-flutter-windows.ps1` → `dist\qualia-flutter-windows-x64\qualia_flutter.exe`

---

### 2026-06-08 — Codex (Sprint 4D ontology routing + symbolic hydration)

**Completed:**
- Added `crates/qualia-client-core/src/ontology_router.rs` to route chat turns toward the most relevant installed ontologies using `id`, `name`, `domain`, and `tags`.
- Extended `OntologyScopeSummary` / chat environment compilation so installed ontologies carry `domain`, `tags`, and `source` metadata into the inference layer.
- Threaded routed `context_namespaces` through `InferenceContextPacket` and `AgentIntent`, and mirrored them into `AgentIntentFrame`.
- Narrowed chat retrieval to the routed ontology subset instead of scanning every installed ontology equally on each turn.
- Updated `orchestrator.rs` so N3 / graph-mutation gating now selects SHACL shapes from routed namespace families (`health`, `legal/guardianship`, `commons`) instead of always using the same default observation shape.
- Added a bounded corrective retry loop in `chat_inference.rs` for deterministic symbolic blocks (`q42:N3Compiler` / SHACL parse failures).

**Verification:**
- `cargo check -p qualia-core-db -p qualia-client-core`
- `cargo test -p qualia-client-core ontology_router -- --nocapture`
- `cargo test -p qualia-core-db orchestrator::tests::test_orchestrator_full_permit_path --lib -- --exact --nocapture`

**Notes for future agents:**
1. `context_namespaces` are now the main bridge between ontology routing and symbolic enforcement. If you extend routing, keep the hashes stable and feed the same family names into orchestrator shape selection.
2. The routed SHACL hydration is deliberately conservative: it uses namespace families to choose the relevant shape set, not full ontology parsing at runtime.
3. The corrective retry loop is single-retry only by design; if you expand it, preserve the bounded behavior so chat turns stay predictable on edge devices.

---

### 2026-06-05 — Claude Sonnet 4.6 (Session 2 — full audit)

**Completed:**
- Full read of: `n3_parser.rs`, `resolver.rs`, `lexicon.rs`, `identifier.rs`,
  `crdt.rs`, `agency.rs`, `webizen.rs` (full), `orchestrator.rs`, `rules.rs`
- Corrected `object` field bit-layout table (was wrong, now matches `resolver.rs` canonical)
- Added §2-B inventory of all non-obvious real implementations
- Documented N3 `RuleType::Defeater` (^>) ↔ `DEFEATER_BIT` linkage (Task G)
- Added §4-D (type tag conflict), §4-E (SHA256 PIN → needs PBKDF2), §4-F (DelegatedAccess alloc)
- Added Task G: N3 → Deontic Quin bridge compiler

**Key findings for future agents:**

1. `resolver.rs` is the canonical object type-tag authority. `logic.rs::extract_float`
   uses the same bit pattern differently — see §4-D.

2. `n3_parser.rs::RuleType::Defeater` (^>) is the surface syntax for what `deontic_logic.rs`
   calls `DEFEATER_BIT`. Task G closes this gap.

3. `execute_vm_frame` in `webizen.rs` is FULLY wired — every `SlgOpcode::Native*` calls
   real implementations. Do not assume these are stubs.

4. `SuspendedTransactionQueue` + `apply_consensus_token` is the M:N signature mechanism
   for multi-party deontic contract ratification. The flow is already tested.

5. `lexicon.rs::generate_60bit_token` masks to 60 bits. All new object field values must
   also mask to 60 bits to keep bits 60-63 free for type tags.

---

### 2026-06-05 — Claude Sonnet 4.6 (Session 1)

**Completed:**
- Full logic modality gap analysis against Gemini's liberal-arts taxonomy
- Implemented `deontic_logic.rs` in full: OP_OBLIGATE/PERMIT/FORBID, DEFEATER_BIT,
  `evaluate_deontic_contract` (zero-heap two-phase scan), `compile_norm_quin`,
  Legal SHACL blueprint (NDA + Guardianship), 10/10 tests passing
- Registered `pub mod deontic_logic` in `lib.rs`
- Created this AGENTS.md, updated HANDOVER.md §7 Roadmap, pushed branch `0.0.12-dev`

**Left incomplete:**
- Tasks A–F above are all unstarted
- The LTL semantic bug (§4-B) is documented but not fixed — left to preserve existing test stability

**Key architectural decision recorded:**
`DEFEATER_BIT = 1u64 << 63` is the q42:unless sentinel in the predicate field. The mask
`0x7FFF_FFFF_FFFF_FF00` strips both the defeater bit and the opcode byte from the predicate
to produce a property-path fingerprint used for defeater matching. All future modalities that
need a sentinel MSB should use the NEXT available MSB (bit 62) and document it here.

**MSB allocation in `predicate`:**
```
bit 63  DEFEATER_BIT    (deontic_logic.rs)
bit 62  [AVAILABLE]     (claim for next modality needing a sentinel)
bit 61  [AVAILABLE]
```

---

### 2026-06-08 - Codex (Bundled ontology fallback + SHACL seeding)

**Completed:**
- Added `crates/qualia-client-core/src/bundled_ontologies.rs` to resolve repo/packaged ontology sources and seed essential ontology artifacts into `{storage}/Index/` on startup.
- Added an offline-first fallback in `resource_import.rs::import_catalog_ontology_with_options` so catalog imports prefer bundled local sources before attempting network download.
- Added tracked `bundled/ontologies/shacl.ttl` and updated the desktop packaging scripts to copy bundled ontology sources into release artifacts alongside `bundled/resources/`.
- Updated Flutter init (`qualia_api.rs::init_core`) to seed bundled ontologies before app features query readiness.

**Verification:**
- `cargo check -p qualia-client-core -p qualia_flutter_rust`
- `cargo test -p qualia-client-core bundled_shacl_source_resolves_when_tracked --lib`

**Notes for future agents:**
1. `DEFAULT_BUNDLED_ONTOLOGIES` is intentionally small right now (`shacl`) and is the place to extend startup-seeded essentials as more local ontology sources are tracked.
2. The import fallback is local-source first for bundled IDs, so packaged apps can stay functional even when a catalog URL 404s or the machine is offline.
3. If a new bundled ontology is added, make sure the packaging scripts continue copying `bundled/ontologies/` into desktop artifacts.

---

## 8. Quick Reference — Running Tests

```powershell
# All engine tests
cargo test -p qualia-core-db --lib

# Specific modality
cargo test -p qualia-core-db --lib deontic_logic
cargo test -p qualia-core-db --lib epistemic      # once Task A is done

# Check compile without tests
cargo check -p qualia-core-db

# Full workspace
cargo test
```

---

### 2026-06-16 - Codex (Zero-heap graph, storage, mesh, and crypto refactor)

**Completed:**
- Reworked `daemon_graph.rs` into a fixed-capacity resident store with deterministic deduplication and no `Vec`/`HashSet` residency in the live daemon graph path.
- Added buffer-first hot-path APIs to `semantic_culler.rs`, `ambient_orchestration.rs`, `csd_storage.rs`, and `acoustic_ble_mesh.rs`, while preserving compatibility wrappers for existing call sites.
- Added zero-heap encryption/decryption and compact key-listing APIs in `specialized_libs/cryptographic_library.rs`, including caller-owned output buffers for AEAD and HKDF operations.
- Promoted `modalities/graph_theory.rs` to a dual-path design: bounded fixed-array analysis for the 10D/edge path, plus quarantined heap-backed batch analysis for compatibility.
- Added bounded graph analysis coverage and resolved the daemon-graph stack overflow by moving the resident store out of stack-initialized construction.

**Verification:**
- `cargo test -p qualia-core-db --lib`

**Notes for future agents:**
1. The preferred graph-analysis entry point is now `analyze_graph_topology_bounded`; treat `analyze_graph_topology` as a cold-path compatibility tool only.
2. The new `*_into` APIs are the intended zero-heap surface for orchestration code. The legacy `Vec`-returning wrappers remain for migration safety, not as the final architecture.
3. `HANDOVER.md` was not present in the workspace during this session, so the older handoff step could not be applied there.

---

### 2026-06-17 — Track B3 polish (Compute Universe / 0.0.17)

**Completed:**
- `sentinel_allows_topology_draft()` — Phase-8 gate on U1→U0 draft batches (0x99 anachronism byte) before `verify_topology_draft_batch`.
- Shared `try_accept_topology_draft()` + `drain_tensor_context_inject()` in `llm_agent.rs` — native threaded and WASM synchronous decode paths now parity-wired (producer start, context inject drain, speculative accept, decode hints).
- Workspace version bump to **0.0.17** (`qualia-core-db`, `qualia-cli`, `qualia-client-core`, and sibling crates).
- Migration plan updated: P-B7 Sentinel gate checked; branch target `0.0.17-dev`.

**Verification:**
- `cargo test -p qualia-core-db --lib`

**Notes for future agents:**
1. Draft denial sets `rollback = true` inline on WASM; native path ORs with `ControlStream` `DenyRollback` from the bifurcated Sentinel thread.
2. α tuning for speculative acceptance ratio is still empirical — see migration plan Appendix F.
3. `HANDOVER.md` still absent; capability inventory update deferred.

---

### 2026-06-27 — Codex (WASM capability profiles + ontology MCP)

**Completed:**
- Added an explicit compile-time WASM capability registry and separated the
  ontology, portal, logic, scientific, LLM, playground, and full profiles.
- Added the isolated `wasm-ontology` kernel and registered
  `crates/webizen-lite-wasm` as the browser-local ontology MCP product.
- Wired 11 bounded MCP tools for N3 inspection, Quin query, SHACL validation,
  modal evaluation, subsumption, hashing, and governance.
- Fixed standalone `wasm-logic`, `wasm-scientific`, and `wasm-full` builds by
  separating semantic/scientific JS exports and gating unsupported modules.
- Made WebGPU an explicit profile dependency instead of an implicit dependency
  of the ontology build.
- Added Pages/release profile checks, ontology package generation, and a
  512 KiB raw / 200 KiB gzip size gate.

**Verification:**
- `cargo test -p webizen-lite-wasm --lib` — 4 passed
- wasm32 checks for `webizen-lite-wasm`, `portal`, `wasm-logic`,
  `wasm-scientific`, `wasm-llm`, and `wasm-full`
- `wasm-pack build crates/webizen-lite-wasm --target web --out-dir pkg --release`
  → 267,993 bytes raw / 94,971 bytes gzip
- `cargo test -p qualia-core-db --lib` compiled and began 2,113 tests, but
  exceeded the 15-minute session limit before producing a final result.

**Notes for future agents:**
1. Add a module to `modalities_lite/mod.rs` only after it passes the ontology
   wasm32 build and introduces no renderer, daemon, network, or filesystem dependency.
2. Keep `wasm_capabilities.rs`, the MCP tool catalog, and
   `docs/manuals/wasm-capability-profiles.md` synchronized.
3. `HANDOVER.md` is absent from the repo, so this session did not recreate or modify it.

---

### 2026-06-30 — Codex (cross-platform volumetric renderer SDK)

**Completed:**
- Lifted `render::gpu::PortalGpu` from a wasm-only module to a cross-platform wgpu 29 renderer.
- Added native offscreen construction on `gpu_context::shared_gpu()`, reusable RGBA8 targets,
  resize support, and caller-owned readback buffers.
- Fixed invalid uniform-array layouts in the ambient, projector, and mesh WGSL shaders.
- Added `webizen_render::VolumetricRenderer` and `RenderScene` adaptation: nodes become Tensor10D
  projector instances; faces/edges become a depth-tested mesh; PNG helpers route through this path.
- Migrated `webizen-render` from wgpu 0.19 to 29, unified spectral colour with the engine oracle,
  and added the verified renderer + studio crates to the workspace.
- Updated the renderer SDK draft to v0.2 and reconciled the standards backlog.

**Verification:**
- `cargo check -p qualia-core-db --lib`
- A2000 hardware gate: native shared-GPU tensor + mesh render and RGBA8 readback passed.
- `cargo test -p webizen-render --lib` — 41 passed.
- `cargo check -p webizen-studio`
- `cargo check -p qualia-core-db --target wasm32-unknown-unknown --no-default-features --features portal`
- Desktop verification was blocked before compilation because uncached Tauri dependencies required
  network access that was unavailable in this session.

**Architectural decisions:**
1. `qualia_core_db::render::gpu` owns the canonical draw graph and device ABI.
2. `webizen-render` is the serde/SDK/image-codec adapter; its immediate-mode renderer is compatibility,
   not a second semantic engine.
3. Native inference and volumetric rendering share `gpu_context::shared_gpu()`; no second adapter is
   requested by the default renderer path.

---

### 2026-06-30 — Codex (workspace dependency modernization)

**Completed:**
- Added the runtime, desktop, and WASM browser crates to the root workspace.
- Updated every independently upgradable direct dependency to its current stable release.
- Removed the wgpu 0.19 dependency graph by migrating `webizen-runtime` to wgpu 29.
- Migrated the desktop shell from Tauri 1 to Tauri 2, including configuration, tray/menu,
  updater-plugin, custom-protocol, webview, event, and sysinfo APIs.
- Migrated minicbor 0.20 to 2.2 and the RustCrypto AEAD stack to aead 0.6 /
  aes-gcm 0.11 / chacha20poly1305 0.11 on native and wasm32 paths.
- Updated PDF, archive, XML, HTTP, configuration, JNI, and browser-WASM dependencies.

**Verification:**
- `cargo outdated --workspace --root-deps-only`: only the wasm32 `getrandom 0.2`
  feature-unification shim required by stable fips20x/x25519 dependencies remains.
- `cargo test -p qualia-core-db --lib` passed.
- `cargo test --workspace --exclude webizen-desktop --no-run` passed.
- `cargo test -p webizen-runtime -p qualia-semantic-library` passed.
- wasm32 checks passed for `qualia-wasm`, `qualia-mobile-harness`, `wellfare-core`,
  `webizen-lite-wasm`, and `webizen-studio`.

**Concurrent-work note:**
- During final verification, another process rewrote
  `crates/webizen-desktop/src/commands/mod.rs` with a large Webizen migration block that
  duplicates command definitions. That work was preserved and excluded from this session's
  commit; it currently prevents a final desktop/workspace build despite the Tauri 2 migration
  compiling successfully immediately before the concurrent rewrite.

---

### 2026-06-30 — Devin (specialized_libs warning audit + implementation sprint)

**Completed:**
- Audited 677 compiler warnings across `specialized_libs/`, distinguishing
  intentional architectural stubs from genuinely dead code.
- Launched 6 parallel deep-dive subagents to identify implementation
  opportunities in crypto, financial, ML, medical, physics, and statistical
  modules. Each found 5+ feasible opportunities.
- Implemented 12 of 13 planned features across 3 modules:

**Cryptographic library** (`specialized_libs/cryptographic_library/mod.rs`):
- Audit log writing: `log_entry()`, `entry_count()`, `entries()` on all 4
  audit log types (Access, Signature, Hash, Proof). Wired into sign/verify/
  hash/proof operations with retention policy enforcement.
- Key relationship tracking: `add_relationship()`, `get_relationships()`,
  `find_related()`, `register_key()`, `add_tag()` on KeyCatalog. Wired
  KeyPair into key generation, RotatedFrom into key rotation.
- Performance metrics: EMA-based `record_*_time()` methods on all 4
  performance optimizers. Wired into all crypto operations.
- Key access policy enforcement: `add_policy()`, `check_permission()`,
  `check_permission_with_context()` on KeyAccessControl. Added
  `get_key_with_access()` to KeyStorage with deny-by-default semantics.
- Key encryption at rest: AES-256-GCM master KEK generation, encrypt/decrypt
  key data with nonce+ciphertext packing.
- 8 new tests (all passing). Warnings: 677 → 669.

**Financial modeling** (`specialized_libs/financial_modeling/`):
- Risk profile validation: compares computed volatility/VaR against
  portfolio's declared RiskTolerance, returns warning if misaligned.
- Portfolio access control: `check_permission()`, `add_access_policy()`,
  audit trail with `log_action()`, `entry_count()`, `entries()`.
  Wired into store_portfolio and get_portfolio.
- Benchmark-based beta/alpha: `compute_risk_metrics()` now accepts
  optional benchmark returns, computes real beta = Cov/Var and
  alpha = mean(R_p) - beta*mean(R_b). No more NaN placeholders.
- Price feed wiring: `register_price_feed()`, `update_price_history()`,
  `ingest_from_feed()`, `apply_to_asset()`, `MarketData::sync_to_assets()`.
- Rebalancing logic: `calculate_drift()`, `rebalance()` with
  RebalanceTrade generation, `PortfolioManager::rebalance_portfolio()`.
- 15 new tests (all passing, 36 total financial tests).

**Machine learning** (`specialized_libs/machine_learning.rs`):
- ModelCache with LRU eviction: `get()` updates access metadata and
  hit/miss stats, `put()` enforces max size with LRU eviction.
- InferenceEngine wired to linear algebra backend: real MLP forward pass
  through Linear layers with activation functions (ReLU, sigmoid, tanh,
  GELU, softmax, SiLU, LeakyReLU, ELU). Unsupported layer types
  (Convolutional, Attention, etc.) return clear error messages.
- 4 new tests (all passing).

**Verification:**
- `cargo test -p qualia-core-db --lib -- cryptographic financial model_cache inference`
  → 186 passed; 0 failed; 1 ignored
- `cargo check -p qualia-core-db` → no errors (669 warnings, down from 677)

**Still pending:**
- ML model loading from GGUF files — completed (real GGUF loading via
  GgufTensorIndex with memmap2, graceful fallback to mock model)
- Physics boundary conditions, CFL time stepping, stencil operators,
  ZNS/CSD data persistence, MeshNetworkManager wiring — all completed
- Statistical ZNS data persistence, Fiduciary Crypto/ZK proof wiring,
  data catalog search, sensitivity analysis for DP — all completed
- Medical HIPAA compliance features (not started this session)

**Updated totals (end of session):**
- 249 tests pass across crypto, financial, ML, physics, statistical modules
- Compiler warnings: 677 → 655 (-22)
- 22 features implemented across 5 modules (crypto: 5, financial: 5,
  ML: 3, physics: 5, statistical: 4)
- 52 new tests added (8 crypto, 15 financial, 8 ML, 13 physics, 11 statistical)

### 2026-07-01 — Codex (linear-algebra privacy engine)

**Completed:**
- Replaced the metadata-only privacy stub with feature-gated pure-Rust BFV:
  standard approximately 128-bit-security parameters, packed signed encryption,
  add, relinearized multiply, rotation-based dot product, caller-buffered
  fixed-point conversion, and verified external ciphertext serialization.
- Added a separate 48-byte `HeCiphertextRef`; BFV ciphertexts and keys never enter
  the `NQuin` payload.
- Added caller-buffered Laplace and calibrated Gaussian DP releases using OS
  entropy, fail-closed budgets, and basic/advanced/RDP composition.
- Split the former file into `privacy/{mod,bfv,differential_privacy}.rs` and
  documented the threat model in `docs/manuals/privacy-engine.md`.

**Verification:**
- Privacy tests: 14 passed, 0 failed, 1 ignored debug-expensive production smoke.
- Production release BFV smoke: 1 passed; degree 4096, key/context cap enforced,
  encrypt/decrypt exact; test execution 2.98s.
- `wasm32-unknown-unknown --no-default-features --features wasm-ontology`: passed.
- Full non-CFD library suite: 2,648 passed, 0 failed, 54 ignored, 7 filtered.
- The unfiltered suite was temporarily blocked by two failures in the separately
  claimed, concurrently developed `engineering_analysis/cfd.rs`; that work was
  preserved and not modified here.

**Architectural decisions:**
1. BFV is used for exact integer/fixed-point algebra; CKKS approximation is not
   mixed into the first privacy ABI.
2. Large cryptographic objects stay in bounded external storage and cross engine
   boundaries only through fixed-size references.
3. The upstream `fhe` implementation is mathematically real but not independently
   audited; it must not be described as FIPS-validated or high-risk-production ready.

### 2026-07-01 — Codex (model-agnostic compression)

**Completed:**
- Replaced `ModelCompression`'s metrics-only behavior with symmetric-int8 PTQ
  over caller-owned buffers, including dequantization, byte ratios, RMSE, and
  maximum reconstruction error.
- Added exact unstructured magnitude pruning and structured output-channel
  pruning. Both emit a packed one-bit keep mask plus retained values rather than
  merely zeroing a dense tensor.
- Added mask-preserving SGD recovery; pruned parameters remain zero throughout
  optimizer updates.
- Added a real teacher-student loop: supported MLP teachers generate targets for
  the existing single-linear-layer SGD student, with optional hard-target
  blending and fidelity measurements before and after training.
- Documented the scope and limitations in
  `docs/manuals/model-compression.md`.

**Verification:**
- Compression end-to-end tests: 5 passed, 0 failed.
- Full `machine_learning` module: 40 passed, 0 failed.
- Full library excluding four concurrently developed CFD tests: 2,655 passed,
  0 failed, 55 ignored.
- The unfiltered suite reaches four pre-existing out-of-bounds failures at
  `engineering_analysis/cfd.rs:510`; that separate uncommitted work was
  preserved and not modified here.

**Architectural decisions:**
1. The generic API operates on flat numeric tensors and is independent of the
   existing GGUF-specific codecs.
2. PTQ is implemented now; QAT remains blocked on fake-quantized backward
   operators and a broader trainer.
3. Distillation is honest about the current training boundary: MLP teacher,
   linear SGD/MSE student. Transformer/CNN and temperature-KL distillation need
   additional training infrastructure.

### 2026-07-13 — Codex (wgpu 30 completion)

**Completed:**
- Finished the wgpu/naga 30 migration across core, extensions, renderer, runtime, and WASM.
- Adopted adapter-intersected timestamp, pipeline-statistics/cache, shader-f16, subgroup, and subgroup-barrier capabilities; experimental cooperative matrices remain explicit opt-in and runtime-oracle-gated.
- Migrated renderer presentation/color-space/vertex-layout APIs and added the missing projector fragment stage exposed by wgpu 30 validation.
- Kept `gemm_f32_tc` f32-faithful and the reduced-precision CUDA WMMA path explicit.
- Replaced unsafe same-process Vulkan/DX12 device benchmarking with deadline-bounded per-adapter worker processes and validated length-delimited CBOR aggregation. CLI and desktop hosts expose private worker entry points; a dedicated worker binary supports other packaging.

**Verification:**
- Core, extensions, renderer, runtime, CLI, worker binary, and WASM checks pass.
- Coopmat capability gating and Stage-4 GEMM selector pass (`max_err=0`).
- Renderer tests: 48 passed; runtime tests: 2 passed.
- Full core library, single-threaded and default-parallel: 5,365 passed, 0 failed, 9 ignored.
- Computational geometry suites: 1,517 passed, 0 failed, 0 ignored.
- Real SmolLM2-360M P64 layer-0 Forge decode matches the CPU oracle (`max_rel=3.28e-6`).
- The remaining nine ignored tests are explicit experimental-hardware paths, performance/manual
  diagnostics, external-tool integration, or an intentionally expensive production BFV smoke;
  they are not unimplemented correctness paths.

**Architectural decisions:**
1. Experimental wgpu features require both operator opt-in and adapter advertisement.
2. Features with no current shader consumer (for example `SHADER_I16`) are not requested speculatively.
3. Each physical adapter/backend benchmark owns its native driver lifetime in a separate bounded process; the parent never successively owns Vulkan and DX12 devices.
4. GPU correctness tests execute by default when their capability is present and share a serialized
   hardware lane so the default parallel suite does not race native driver/context lifetimes.
