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

object     [63]      Type tag: 0x0 = IRI hash, 0x1 = f32 (bits 0-31), 0x2 = u32, …
           [0..62]   Value / target entity hash or tagged scalar

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
