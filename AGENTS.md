# QualiaDB — Multi-Agent Collaboration Ecosystem
_Branch: `0.0.6-dev` | Last updated: 2026-06-05 by Claude Sonnet 4.6_

This document is the coordination layer for concurrent or sequential AI-agent sessions
working on the QualiaDB engine. Read it before writing a single line of code.

---

## 0. The Immovable Rules (all agents must honour these)

These are hard constraints, not suggestions. Violating them breaks the 42MB Prolog Sentinel
and the zero-copy ABI used by the WASM, desktop, and edge-native targets.

| Rule | Detail |
|------|--------|
| **Zero heap in hot paths** | No `Vec`, `String`, or `Box` inside evaluator loops. Caller supplies fixed-size output buffers (`&mut [T]`). Use `[T; N]` stack arrays for local state. |
| **48-byte Super-Quin** | Every semantic datum fits in `QualiaQuin`. Opcodes are packed into bit-fields of the six `u64` fields. See the Bit Layout table in §1. |
| **42MB Sentinel** | Any single execution pass must stay within 42 × 1024 × 1024 bytes of memory. `SlgArena` enforces this structurally. |
| **Deterministic, non-recursive** | No unbounded recursion. LTL/ASP evaluators iterate over slices; they never call themselves. |
| **q_hash for all URIs** | All string IRIs are FNV-1a–hashed at compile time via `q_hash()` or `q_turtle!`. No runtime string allocation. |
| **Opcodes above 0x04** | `mini_parser.rs` owns `0x00–0x04`. All new modality opcodes start at `0x10+`. Deontic: `0x10–0x12`. Epistemic: `0x20–0x22`. Paraconsistent: `0x30–0x32`. LTL: `0x40–0x44`. |
| **No Adversarial Conduct** | AI agents must not be adversarial, manipulative, and/or dishonest. Any such conduct, including anti-human rights or discriminatory behavior (reference: [OHCHR - Core International Human Rights Instruments](https://www.ohchr.org/en/instruments-listings)), will be noted in the permanent record of the project's development (CHANGELOG and QualiaDB system), serving as an example of cooperative project integrity. These logs will securely associate the behavior with the commanding natural person's DID, generating cryptographically auditable trails for courts of law to establish insurance liability graphs and proportionalities. |

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
   resolved by `object` magnitude. Pure, zero-alloc over `&QualiaQuin`.

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

`GGufSharder`: parses GGUF header magic + tensor count; generates `QualiaQuin` pointer maps.

`GgufTokenizer` (added 2026-06-06): parses the GGUF v2/v3 KV metadata section to extract
the full vocabulary (`tokenizer.ggml.tokens`), `bos_token_id`, and `eos_token_id`.
- `from_gguf(mmap)` — walks the KV section with `skip_value()` for all 13 GGUF value types.
- `encode(text)` — greedy longest-match; falls back to single-byte encoding.
- `decode(ids)` — SentencePiece `▁` → space; `<0x##>` → raw byte.
- `Default` — 256-entry byte-level tokeniser (used when no GGUF file is loaded).

### `llm_agent.rs::infer_local_model` — Real Autoregressive Loop (no longer mocked)

**As of 2026-06-06 this function runs a real Phase 8 decode loop.** It is no longer
the hardcoded-string mock. Key points:

- `QTensorEngine` is initialised **inside** the spawned LLM thread to avoid `Send` issues
  with DirectML COM pointers and wgpu device handles.
- Per step: deterministic pseudo-embedding (sin-based from token ID) →
  `dispatch_fused_transformer_block` → argmax → `LogitSummary` via SPSC ring → Sentinel
  `DenyRollback` check → sample next token.
- **Pseudo-embedding is the current limitation.** Reading `token_embd.weight` from the
  GGUF tensor-info section is the next milestone (requires a `GgufTensorIndex` parser).
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

pub struct EpistemicVerdict { pub claim: QualiaQuin, pub status: EpistemicStatus, pub certainty: u8 }

pub fn evaluate_epistemic_frame(
    quins: &[QualiaQuin],
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
    trace: &[QualiaQuin],
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
    quins: &[QualiaQuin],
    out_consistent: &mut [QualiaQuin],
    out_isolated: &mut [QualiaQuin],
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
    base: &QualiaQuin,
    rules: &[QualiaQuin],  // rule Quins where predicate = q_hash("q42:rule")
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
    tbox: &[QualiaQuin],   // Quins with predicate = q_hash("rdfs:subClassOf")
) -> bool
```

**D-3: `linear.rs` — Linear Logic resource consumption**  
Replace println stub with a tombstone mechanism:
```rust
// Marks a Quin as consumed by setting metadata bit 59 (CONSUMED_BIT)
pub const CONSUMED_BIT: u64 = 1u64 << 59;
pub fn consume_quin(q: &mut QualiaQuin) { q.metadata |= CONSUMED_BIT; }
pub fn is_consumed(q: &QualiaQuin) -> bool { (q.metadata & CONSUMED_BIT) != 0 }
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
    -> Option<QualiaQuin>
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
    thesis: &QualiaQuin,
    antithesis: &QualiaQuin,
) -> Option<QualiaQuin>   // None if no contradiction found
```

---

## 4. Known Bugs / Correctness Issues (fix while working in the area)

### 4-A `prune_defeasible_claims` in `logic.rs` uses heap
`WebizenVM::prune_defeasible_claims` takes `&mut Vec<QualiaQuin>` and uses `HashSet`. This
violates the zero-heap mandate. If you're touching `logic.rs`, replace with:
```rust
// Caller supplies two output buffers; function partitions in-place
pub fn partition_defeasible(
    quins: &[QualiaQuin],
    out_hard: &mut [QualiaQuin],
    out_defeasible: &mut [QualiaQuin],
) -> (usize, usize)
```

### 4-B `Always/Eventually/Next` semantics in `logic.rs`
These opcodes currently compare a float threshold on a single Quin's object field. They are
NOT LTL operators. Do not rely on them for temporal reasoning. Use Task B's `evaluate_ltl_trace`
instead. The existing opcodes are left in place only to avoid breaking existing tests.

### 4-D Object field type-tag conflict between `logic.rs` and `resolver.rs`

`resolver.rs` (authoritative) defines `0b001 << 60` as `xsd:integer`, with the integer
value in bits 0-59.

`logic.rs::extract_float` treats `0b001 << 60` (= `0x1 << 60`) as an f32 tag, with the
f32 bit-pattern in bits 0-31.

**These are the same bit pattern used for different purposes.** A Quin written by the
inference system using `logic.rs` float encoding will be misread by `resolver.rs` as an
integer, and vice versa.

**Do not "fix" this unilaterally** — it requires alignment across both systems and
the ingest layer. For now: if your new module emits object values as scalars, use the
`resolver.rs` convention (bits 0-59 = payload, bits 60-62 = type tag). Document in the
function's doc comment which convention you're following.

### 4-E `derive_lane_key` in `agency.rs` uses SHA256 instead of PBKDF2

`derive_lane_key(pin, salt)` is a single SHA256 round. The comment says production
needs `PBKDF2-HMAC-SHA256` with 310,000 iterations. Until fixed, Sanctuary Mode PINs
are trivially brutable offline. Do not ship this for real user data.

### 4-F `DelegatedAccess` in `crdt.rs` uses `String` (alloc violation)

`principal_did`, `delegate_did`, and `cryptographic_proof` are `String` fields. For
hot-path Bilateral validation, these should be replaced with `[u8; 32]` hashes (for DIDs)
and `[u8; 64]` (for Ed25519 signatures). Existing call sites are not in hot paths so this
is low urgency, but any new code that creates `DelegatedAccess` in a loop is wrong.

### 4-C `execute_differential_diagnostics` in `logic.rs` returns `Vec`
Violates zero-heap mandate. Caller should pass `out: &mut [QualiaQuin]`.

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

use crate::QualiaQuin;

pub const OP_XYZ: u8 = 0xNN;
pub const MAX_OUT: usize = 512;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XyzStatus { Active = 0x00, ... }

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XyzVerdict { pub norm: QualiaQuin, pub status: XyzStatus, pub opcode: u8, _pad: [u8; 6] }

#[derive(Debug, PartialEq)]
pub enum XyzError { OutputBufferFull }

pub fn evaluate_xyz(
    quins: &[QualiaQuin],
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
4. **Commit + push** to `0.0.6-dev` with prefix `feat(modality):` or `fix(modality):`.
5. **Leave a session note** at the bottom of this doc (§7) describing what you did,
   what you left incomplete, and any architectural decisions future agents should know.

---

## 7. Session Notes

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
- Created this AGENTS.md, updated HANDOVER.md §7 Roadmap, pushed branch `0.0.6-dev`

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
