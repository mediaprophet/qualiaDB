//! Deontic Logic extension for the Qualia Bytecode VM.
//!
//! Implements a defeasible deontic contract evaluator over a `&[NQuin]` slice.
//! Conforms to the 42 MB Prolog Sentinel memory ceiling, the 48-byte Super-Quin
//! invariant, and the zero-heap-allocation mandate (no `Vec`, `String`, or `Box`).
//!
//! # Opcodes
//!
//! Three raw `u8` constants define the deontic modality, packed into **bits 0–7** of
//! the `predicate` field of every norm Quin:
//!
//! | Constant      | Value | SDL formula | Meaning                          |
//! |---------------|-------|-------------|----------------------------------|
//! | `OP_OBLIGATE` | 0x10  | O(φ)        | Party *must* perform action φ    |
//! | `OP_PERMIT`   | 0x11  | P(φ)        | Party *may* perform action φ     |
//! | `OP_FORBID`   | 0x12  | F(φ)=O(¬φ)  | Party *must not* perform action φ|
//!
//! # 48-byte Norm Quin Layout
//!
//! ```text
//! ┌──────────┬──────────────────────────────────────────────────────────────────┐
//! │ Field    │ Bit layout                                                       │
//! ├──────────┼──────────────────────────────────────────────────────────────────┤
//! │ subject  │ [63]=0 (rsvd)  │ [0..62] = FNV-1a hash of the bound party DID  │
//! │ predicate│ [63]=DEFEATER  │ [8..62] = property-path hash (action/norm URI) │
//! │          │                │ [0..7]  = deontic opcode (OP_OBLIGATE etc.)    │
//! │ object   │ [63]=0 (rsvd)  │ [0..62] = FNV-1a hash of the action object    │
//! │ context  │ [56..63] = sensitivity class (from NQuin::SENSITIVITY_*)   │
//! │          │ [0..55]  = q_hash of the contract/graph DID                     │
//! │ metadata │ [61..62] = PermissiveRoutingLane bits                           │
//! │          │ [32..60] = Lamport logical clock                                │
//! │          │ [0..31]  = expiry as truncated Unix-32 timestamp                │
//! │ parity   │ XOR fold of subject ⊕ predicate ⊕ object ⊕ context (ECC check)  │
//! └──────────┴──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Defeater Nodes — `q42:unless`
//!
//! A Quin with **bit 63 of `predicate` set** (`DEFEATER_BIT`) is not a primary norm;
//! it is a `q42:unless` exception node that defeats any norm sharing the same
//! (subject, context, property-path) fingerprint.  This supports non-monotonic,
//! defeasible reasoning:
//!
//! > *Alice is forbidden from disclosing project data — **unless** she is speaking
//! > to a certified auditor.*
//!
//! The evaluator performs a two-phase linear scan:
//! 1. **Defeater harvest** — collect up to `MAX_DEFEATER_SLOTS` fingerprints into
//!    a fixed `[u64; 64]` stack buffer (512 bytes, one cache-line group on Cortex-A78).
//! 2. **Norm evaluation** — for each non-defeater Quin: check expiry, probe the
//!    defeater buffer, emit a `DeonticVerdict` into the caller-supplied `out` slice.
//!
//! # Legal SHACL Blueprint
//!
//! ## Non-Disclosure Agreement (NDA)
//!
//! NDA between `did:web:alice.example` and `did:web:bob.example`, covering
//! confidential project-X data, valid until 2028-01-01 (Unix epoch 1 830 297 600).
//! Three Quins fully encode the agreement and its auditor exception:
//!
//! ```text
//! // Quin 1 — Alice's confidentiality prohibition
//! subject   = q_hash("did:web:alice.example")
//! predicate = OP_FORBID as u64
//!           | (q_hash("q42:disclose") << 8)           // property-path in [8..62]
//! object    = q_hash("q42:data:project-x:confidential")
//! context   = q_hash("did:web:nda:contract-001")      // contract graph
//! metadata  = 1_830_297_600_u64                        // expiry in bits [0..31]
//! parity    = subject ^ predicate ^ object ^ context   // ECC fold
//!
//! // Quin 2 — Bob's symmetric prohibition (identical structure, different subject)
//! subject   = q_hash("did:web:bob.example")
//! predicate = OP_FORBID as u64 | (q_hash("q42:disclose") << 8)
//! object    = q_hash("q42:data:project-x:confidential")
//! context   = q_hash("did:web:nda:contract-001")
//! metadata  = 1_830_297_600_u64
//! parity    = subject ^ predicate ^ object ^ context
//!
//! // Quin 3 — Defeater: Alice MAY disclose to a certified auditor (q42:unless)
//! subject   = q_hash("did:web:alice.example")
//! predicate = DEFEATER_BIT                             // bit 63 marks q42:unless
//!           | OP_PERMIT as u64
//!           | (q_hash("q42:disclose") << 8)           // same property-path as Quin 1
//! object    = q_hash("q42:role:certified-auditor")    // excepted entity class
//! context   = q_hash("did:web:nda:contract-001")      // same contract graph
//! metadata  = 1_830_297_600_u64
//! parity    = subject ^ predicate ^ object ^ context
//! ```
//!
//! Quin 3 shares (subject, context, property-path) with Quin 1, so the evaluator
//! marks Quin 1 as `DeonticStatus::Defeated` when an auditor invokes the exception.
//!
//! ## Guardianship Contract
//!
//! Ward: `did:web:ward.example`, Guardian: `did:web:guardian.example`.
//! Guardianship expires at majority (2030-01-01, epoch 1 893 456 000):
//!
//! ```text
//! // Quin 1 — Guardian obligated to act in the ward's best interest
//! subject   = q_hash("did:web:guardian.example")
//! predicate = OP_OBLIGATE as u64
//!           | (q_hash("q42:actInBestInterest") << 8)
//! object    = q_hash("did:web:ward.example")
//! context   = q_hash("did:web:guardianship:contract-002")
//! metadata  = 1_893_456_000_u64   // contract expires when ward reaches majority
//! parity    = subject ^ predicate ^ object ^ context
//!
//! // Quin 2 — Temporal defeater: ward may self-determine after majority age
//! subject   = q_hash("did:web:ward.example")
//! predicate = DEFEATER_BIT
//!           | OP_PERMIT as u64
//!           | (q_hash("q42:actInBestInterest") << 8)  // defeats the same obligation path
//! object    = q_hash("did:web:ward.example")
//! context   = q_hash("did:web:guardianship:contract-002")
//! metadata  = 1_893_456_000_u64   // carries same timestamp; expiry semantics applied
//! parity    = subject ^ predicate ^ object ^ context
//! ```
//!
//! After majority, `now_unix > 1_893_456_000` causes Quin 1 to emit
//! `DeonticStatus::Expired`, and the defeater Quin 2 itself becomes moot —
//! demonstrating how temporal bounds compose naturally with defeasibility in a
//! single linear scan without branching stacks.
//!
//! # Edge-native 3-core CPU triad
//!
//! The two-phase design maps to the triad naturally:
//! - **Core 0** — defeater harvest (Phase 1, read-only, highly prefetchable).
//! - **Core 1** — norm evaluation (Phase 2, linear probe of the 512-byte buffer).
//! - **Core 2** — verdict dispatch / downstream enforcement routing.
//!
//! Cache-line pressure is bounded: the `[u64; MAX_DEFEATER_SLOTS]` buffer fits in
//! 8 × 64-byte cache lines; each `DeonticVerdict` is 64 bytes (one cache line).

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
use crate::modalities::logic::n3_parser::Term;
use crate::q_hash;
use crate::NQuin;

// ─── Deontic Opcodes ─────────────────────────────────────────────────────────
//
// These u8 constants are packed into bits [0..7] of the `predicate` field of
// every norm Quin.  Values 0x10–0x12 are chosen above the mini_parser opcode
// range (0x00–0x04) to allow mixed Quin databases without collision.

/// O(φ) — the subject party *must* perform the action.
pub const OP_OBLIGATE: u8 = 0x10;

/// P(φ) — the subject party *may* perform the action.
pub const OP_PERMIT: u8 = 0x11;

/// F(φ) = O(¬φ) — the subject party *must not* perform the action.
pub const OP_FORBID: u8 = 0x12;

// ─── SDL⁺ extension opcodes (deontic block 0x13–0x1F, per DEONTIC_LOGIC_PLAN §3) ──

/// U(φ) — optionality / indifference: `¬O(φ) ∧ ¬F(φ)`. The system is indifferent;
/// neither doing nor omitting φ is a violation. May be asserted or derived
/// (see [`is_optional`]).
pub const OP_OPTIONAL: u8 = 0x13;

/// G(φ) — gratuitousness: `¬O(φ)`. The agent is free to omit φ (it may still be
/// permitted or forbidden). May be asserted or derived (see [`is_gratuitous`]).
pub const OP_GRATUITOUS: u8 = 0x14;

/// O(q | p) — the head of a dyadic / conditional obligation: q is obligatory
/// *given* condition p. Evaluation is fact-driven (see [`evaluate_conditional_obligation`]);
/// contrary-to-duty is the special case p = "primary breached".
pub const OP_CONDITIONAL: u8 = 0x15;

/// Reserved for Phase 3 (STIT agency): `O[α stit φ]`. Declared here to fence the
/// opcode so nothing else claims it before agency lands.
pub const OP_STIT: u8 = 0x16;

/// An *undercutting* defeater: combined with [`DEFEATER_BIT`] it invalidates the
/// inference link `p ⇒ Oq` without asserting `¬Oq` (vs a *rebutting* defeater — a
/// `DEFEATER_BIT` node with an O/P/F opcode — which asserts the contrary). The
/// fingerprint match is identical (the opcode byte is masked out); only the
/// classification in [`DefeatKind`] differs.
pub const OP_UNDERCUT: u8 = 0x17;

/// Bit 63 of `predicate`: marks a `q42:unless` defeater / exception node.
/// When set the Quin is *not* a primary norm and defeats matching obligations.
/// Canonical bit position lives in the FrameLayout ABI (single source of truth).
pub use crate::frame_layout::DEFEATER_BIT;

/// Stack capacity for defeater fingerprints per evaluation call.
/// 64 slots × 8 bytes = 512 bytes — fits within a single L1 cache-line group.
pub const MAX_DEFEATER_SLOTS: usize = 64;

// ─── DeonticStatus ────────────────────────────────────────────────────────────

/// The result of evaluating a single norm Quin against temporal bounds and defeaters.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeonticStatus {
    #[default]
    /// Norm is temporally valid and has no active defeater.
    Active = 0x00,
    /// A matching `q42:unless` defeater node was found; obligation is overridden.
    Defeated = 0x01,
    /// Current timestamp exceeds the expiry embedded in `metadata[0..31]`.
    Expired = 0x02,
    /// The Quin's predicate carries an unrecognised opcode byte; skipped by caller.
    Malformed = 0x03,
    /// Norm is parsed and valid, but its effectivity window has not yet begun
    /// (`now < effective_from`). Not yet binding. (Lifecycle, Phase 1.)
    Pending = 0x04,
    /// An in-force obligation whose action was not performed (or a prohibition that
    /// was breached), per the supplied facts. Triggers CTD / sanction routing.
    Violated = 0x05,
    /// An in-force obligation that has been fulfilled per the supplied facts; the
    /// specific duty terminates.
    Discharged = 0x06,
}

/// How a norm came to be [`DeonticStatus::Defeated`] — Hart/Pollock's rebutting vs
/// undercutting distinction. A *rebutting* defeater asserts the contrary conclusion
/// (`DEFEATER_BIT` + an O/P/F opcode); an *undercutting* defeater ([`OP_UNDERCUT`])
/// severs the rule's support without asserting the contrary.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DefeatKind {
    #[default]
    /// Not defeated.
    None = 0x00,
    /// Defeated by a contrary norm (rebutting).
    Rebutting = 0x01,
    /// Defeated by link-invalidation (undercutting).
    Undercutting = 0x02,
}

// ─── DeonticVerdict ───────────────────────────────────────────────────────────

/// A verdict emitted for one norm Quin.  Exactly 64 bytes — one cache line.
///
/// Layout: 48-byte norm Quin + 1-byte status + 1-byte opcode + 6-byte pad = 56 B
/// aligned to 8, padded by the compiler to the nearest power-of-two boundary.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeonticVerdict {
    /// The original norm Quin that was evaluated.
    pub norm: NQuin,
    /// Outcome of the evaluation.
    pub status: DeonticStatus,
    /// Deontic opcode extracted from `norm.predicate[0..7]`.
    pub opcode: u8,
    /// When `status == Defeated`, *how* it was defeated (rebutting vs undercutting);
    /// `None` otherwise.
    pub defeat_kind: DefeatKind,
    _pad: [u8; 5],
}

// ─── DeonticError ─────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum DeonticError {
    /// The caller-supplied `out` slice was exhausted before the scan completed.
    OutputBufferFull,
}

// ─── Bit-field helpers ────────────────────────────────────────────────────────

/// Extracts the deontic opcode from bits [0..7] of a `predicate` word.
#[inline(always)]
pub fn extract_deontic_opcode(predicate: u64) -> u8 {
    (predicate & 0xFF) as u8
}

/// Extracts the 32-bit expiry from bits [0..31] of a `metadata` word.
/// A zero value means "no expiry set" and is always treated as valid.
#[inline(always)]
pub fn extract_expiry_unix32(metadata: u64) -> u32 {
    (metadata & 0xFFFF_FFFF) as u32
}

/// Produces the defeater-matching fingerprint for any Quin (norm or defeater).
///
/// Two Quins share a fingerprint iff they bind the same party (`subject`),
/// the same contract graph (`context`), and the same property-path
/// (`predicate[8..62]` — the portion above the opcode byte and below the
/// defeater bit).  The opcode byte and defeater bit are masked out so that a
/// `q42:unless` node correctly matches the norm it defeats.
#[inline(always)]
pub fn defeater_fingerprint(q: &NQuin) -> u64 {
    // Strip defeater bit (63) and opcode byte (0..7); retain property-path (8..62).
    let path_bits = q.predicate & 0x7FFF_FFFF_FFFF_FF00;
    q.subject ^ q.context ^ path_bits
}

/// Harvest `q42:unless` defeater fingerprints from a contract slice (Phase 1 only).
pub fn harvest_defeater_fingerprints(quins: &[NQuin], out: &mut [u64]) -> usize {
    let mut count = 0usize;
    for &q in quins {
        if q.predicate & DEFEATER_BIT == 0 {
            continue;
        }
        let expected_parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        if q.parity == expected_parity && count < out.len() {
            out[count] = defeater_fingerprint(&q);
            count += 1;
        }
    }
    count
}

/// Returns `true` if the defeater buffer contains a fingerprint that matches `norm`.
#[inline]
pub fn norm_has_active_defeater(norm: &NQuin, defeaters: &[u64]) -> bool {
    has_defeater(defeaters, norm)
}

#[inline]
fn has_defeater(defeaters: &[u64], norm: &NQuin) -> bool {
    let key = defeater_fingerprint(norm);
    let mut i = 0;
    while i < defeaters.len() {
        if defeaters[i] == key {
            return true;
        }
        i += 1;
    }
    false
}

/// Like [`has_defeater`], but returns *which kind* of defeater matched (rebutting vs
/// undercutting), or [`DefeatKind::None`] if the norm is undefeated. `kinds[i]` is the
/// kind of `defeaters[i]` (parallel arrays harvested together).
#[inline]
fn defeater_kind_for(defeaters: &[u64], kinds: &[DefeatKind], norm: &NQuin) -> DefeatKind {
    let key = defeater_fingerprint(norm);
    let mut i = 0;
    while i < defeaters.len() {
        if defeaters[i] == key {
            return kinds[i];
        }
        i += 1;
    }
    DefeatKind::None
}

// ─── evaluate_deontic_contract ────────────────────────────────────────────────

/// Evaluate a deontic contract encoded as a `&[NQuin]` slice.
///
/// ## Algorithm
///
/// **Phase 1 — Defeater harvest** (single forward pass, O(n)):
/// Every Quin whose `predicate` has `DEFEATER_BIT` set is identified as a
/// `q42:unless` defeater.  Its fingerprint is written into the fixed-capacity
/// `[u64; MAX_DEFEATER_SLOTS]` stack buffer.  Excess defeaters beyond
/// `MAX_DEFEATER_SLOTS` are silently dropped (contracts this large exceed the
/// 42 MB Prolog Sentinel and are rejected at ingest time).
///
/// **Phase 2 — Norm evaluation** (single forward pass, O(n)):
/// Every Quin whose `predicate[0..7]` ∈ {`OP_OBLIGATE`, `OP_PERMIT`, `OP_FORBID`}
/// and whose `DEFEATER_BIT` is **clear** is a primary norm.  For each:
/// 1. Temporal check: if `expiry != 0 && now_unix > expiry` → `Expired`.
/// 2. Defeater probe: if `has_defeater(buffer, quin)` → `Defeated`.
/// 3. Otherwise → `Active`.
///
/// Non-deontic Quins (opcode not in the set above) are skipped silently.
///
/// ## Constraints
///
/// - Zero heap allocation: all state lives in registers and the caller-supplied
///   `out` slice.
/// - Stack budget: `8 × MAX_DEFEATER_SLOTS` bytes (512 B) + frame overhead.
/// - Deterministic O(n²) worst-case defeater probe, O(n) amortised for contracts
///   with few exceptions (the common case in legal documents).
///
/// ## Parameters
///
/// * `quins`     — the deontic contract encoded as a Quin slice.
/// * `now_unix`  — current time as a truncated 32-bit Unix timestamp.
/// * `out`       — caller-supplied verdict buffer; must be `≥` the number of
///                 norm Quins in `quins` to avoid `OutputBufferFull`.
///
/// ## Returns
///
/// `Ok(n)` where `n` is the number of verdicts written to `out[..n]`.
pub fn evaluate_deontic_contract(
    quins: &[NQuin],
    now_unix: u32,
    out: &mut [DeonticVerdict],
) -> Result<usize, DeonticError> {
    // ── Phase 1: harvest defeater fingerprints ─────────────────────────────────
    //
    // Stack-allocated; fits in < 1 KB, well within any thread stack.
    let mut defeater_buf = [0u64; MAX_DEFEATER_SLOTS];
    let mut kind_buf = [DefeatKind::Rebutting; MAX_DEFEATER_SLOTS];
    let mut defeater_count = 0usize;

    for &q in quins {
        if q.predicate & DEFEATER_BIT != 0 {
            // ECC Parity XOR fold check
            let expected_parity = q.subject ^ q.predicate ^ q.object ^ q.context;
            if q.parity == expected_parity {
                if defeater_count < MAX_DEFEATER_SLOTS {
                    defeater_buf[defeater_count] = defeater_fingerprint(&q);
                    // OP_UNDERCUT severs the rule link; any other opcode rebuts.
                    kind_buf[defeater_count] = if extract_deontic_opcode(q.predicate) == OP_UNDERCUT
                    {
                        DefeatKind::Undercutting
                    } else {
                        DefeatKind::Rebutting
                    };
                    defeater_count += 1;
                }
                // Excess defeaters are dropped; contracts this dense are rejected upstream.
            }
        }
    }

    let active_defeaters = &defeater_buf[..defeater_count];
    let active_kinds = &kind_buf[..defeater_count];

    // ── Phase 2: evaluate norm Quins ──────────────────────────────────────────
    let mut verdict_count = 0usize;

    for &q in quins {
        // Defeater nodes are not norms; skip them in the second pass.
        if q.predicate & DEFEATER_BIT != 0 {
            continue;
        }

        let expected_parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        if q.parity != expected_parity {
            if verdict_count >= out.len() {
                return Err(DeonticError::OutputBufferFull);
            }
            out[verdict_count] = DeonticVerdict {
                norm: q,
                status: DeonticStatus::Malformed,
                opcode: extract_deontic_opcode(q.predicate),
                defeat_kind: DefeatKind::None,
                _pad: [0u8; 5],
            };
            verdict_count += 1;
            continue;
        }

        let opcode = extract_deontic_opcode(q.predicate);

        let mut defeat_kind = DefeatKind::None;
        let status = match opcode {
            OP_OBLIGATE | OP_PERMIT | OP_FORBID => {
                let expiry = extract_expiry_unix32(q.metadata);
                if expiry != 0 && now_unix > expiry {
                    DeonticStatus::Expired
                } else {
                    let k = defeater_kind_for(active_defeaters, active_kinds, &q);
                    if k != DefeatKind::None {
                        defeat_kind = k;
                        DeonticStatus::Defeated
                    } else {
                        DeonticStatus::Active
                    }
                }
            }
            // Not a deontic Quin — skip silently (e.g. SHACL shape Quins coexist).
            _ => continue,
        };

        if verdict_count >= out.len() {
            return Err(DeonticError::OutputBufferFull);
        }

        out[verdict_count] = DeonticVerdict {
            norm: q,
            status,
            opcode,
            defeat_kind,
            _pad: [0u8; 5],
        };
        verdict_count += 1;
    }

    Ok(verdict_count)
}

// ─── N3 → deontic norm bridge ───────────────────────────────────────────────

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub fn term_uri_hash(term: &Term) -> Option<u64> {
    crate::modalities::logic::n3_parser::term_uri_hash(term)
}

// Canonical `values:` deontic operators. The registry stores a compiled rule
// (hashes only - the predicate IRI string is gone), so the deontic opcode is
// recovered by matching the premise predicate hash against these. Both the full
// IRI and the CURIE token are listed, because `@prefix` is not expanded on the
const FORBID_HASHES: [u64; 2] = [
    q_hash("https://ns.webcivics.net/values/forbids"),
    q_hash("values:forbids"),
];
const PERMIT_HASHES: [u64; 2] = [
    q_hash("https://ns.webcivics.net/values/permits"),
    q_hash("values:permits"),
];
const OBLIGATE_HASHES: [u64; 4] = [
    q_hash("https://ns.webcivics.net/values/requires"),
    q_hash("values:requires"),
    q_hash("https://ns.webcivics.net/values/obligates"),
    q_hash("values:obligates"),
];

/// Classify a premise-predicate hash into a deontic opcode (+ defeater flag).
///
/// A `Defeater` (`^>`) rule is always a `q42:unless` permit-defeater. Otherwise a
/// recognised `values:` operator picks the opcode; an unrecognised predicate
/// falls back to the rule-type default (Strict/Linear => obligation, Defeasible =>
/// permission), preserving behaviour for non-`values` contract predicates.
fn opcode_from_predicate_hash(pred_hash: u64, rule_type: crate::modalities::logic::n3_parser::RuleType) -> (u8, bool) {
    use crate::modalities::logic::n3_parser::RuleType;
    if matches!(rule_type, RuleType::Defeater) {
        return (OP_PERMIT, true);
    }
    if FORBID_HASHES.contains(&pred_hash) {
        return (OP_FORBID, false);
    }
    if PERMIT_HASHES.contains(&pred_hash) {
        return (OP_PERMIT, false);
    }
    if OBLIGATE_HASHES.contains(&pred_hash) {
        return (OP_OBLIGATE, false);
    }
    match rule_type {
        RuleType::Strict | RuleType::Linear => (OP_OBLIGATE, false),
        RuleType::Defeasible => (OP_PERMIT, false),
        RuleType::Defeater => (OP_PERMIT, true),
    }
}

/// Compile an N3 [`Rule`] into a norm Quin (or defeater Quin for `^>` rules).
///
/// Maps premise triple → party / property / action; `rule_type` → opcode + defeater flag.
pub fn compile_n3_rule_to_norm(
    rule: &crate::modalities::logic::n3_compiler::CompiledRule,
    contract_hash: u64,
    expiry_unix32: u32,
) -> Option<NQuin> {
    // `triples` is a fixed `[_; 8]` array, so `.first()` is always `Some`; an
    // empty rule must be rejected on `len`, not on `.first()`.
    if rule.premise.len == 0 {
        return None;
    }
    let premise = rule.premise.triples.first()?;
    let party = premise.subject.as_u64();
    let property_path = premise.predicate.as_u64();
    let action_object = premise.object.as_u64();

    // Recover the deontic opcode from the premise predicate hash (the compiled
    // rule no longer carries the IRI string).
    let (opcode, is_defeater) = opcode_from_predicate_hash(property_path, rule.rule_type);

    Some(compile_norm_quin(
        party,
        opcode,
        property_path,
        action_object,
        contract_hash,
        expiry_unix32,
        is_defeater,
    ))
}

// ─── compile_norm_quin ────────────────────────────────────────────────────────

/// Build a norm Quin from its logical components.
///
/// Convenience constructor that packs the deontic opcode and property-path hash
/// into `predicate`, stores the contract DID in `context`, and sets the ECC
/// parity to the XOR fold of the four semantic fields.
///
/// # Parameters
/// * `party_did_hash`    — `q_hash` of the bound party's DID.
/// * `opcode`            — `OP_OBLIGATE`, `OP_PERMIT`, or `OP_FORBID`.
/// * `property_path_hash`— `q_hash` of the obligation/action URI.
/// * `action_object_hash`— `q_hash` of the action's target entity/data.
/// * `contract_hash`     — `q_hash` of the contract graph DID.
/// * `expiry_unix32`     — 32-bit Unix timestamp for the norm's expiry (0 = no expiry).
/// * `is_defeater`       — when `true`, sets `DEFEATER_BIT` making this a `q42:unless` node.
#[inline]
pub fn compile_norm_quin(
    party_did_hash: u64,
    opcode: u8,
    property_path_hash: u64,
    action_object_hash: u64,
    contract_hash: u64,
    expiry_unix32: u32,
    is_defeater: bool,
) -> NQuin {
    let defeater_flag = if is_defeater { DEFEATER_BIT } else { 0u64 };
    // Mask DEFEATER_BIT from the shifted path so only `is_defeater` controls bit 63.
    let path_bits = (property_path_hash << 8) & !DEFEATER_BIT;
    let predicate = defeater_flag | path_bits | (opcode as u64);
    let metadata = expiry_unix32 as u64; // bits [0..31]; Lamport/routing bits left zero
    let parity = party_did_hash ^ predicate ^ action_object_hash ^ contract_hash;

    NQuin {
        subject: party_did_hash,
        predicate,
        object: action_object_hash,
        context: contract_hash,
        metadata,
        parity,
    }
}

// ─── Contrary-to-duty (dyadic deontic) ──────────────────────────────────────────

/// Contrary-to-duty obligation `O(reparation / breach)`: a *secondary* obligation
/// that arises precisely because a *primary* obligation was breached (the
/// remedy/reparation logic — Geneva/ICCPR remedy instruments). Returns `true` iff
/// the CTD is satisfied: either the primary was NOT breached by the party (the
/// CTD is not triggered), or it was breached AND the reparation has been fulfilled.
///
/// Facts convention: a breach is `(party, q42:breached, primary)`; a fulfilled
/// reparation is `(party, q42:fulfilled, reparation)`. Zero-heap (linear scans).
pub fn evaluate_contrary_to_duty(
    facts: &[NQuin],
    party: u64,
    primary: u64,
    reparation: u64,
) -> bool {
    // CTD is the dyadic obligation O(reparation | breached(primary)).
    evaluate_conditional_obligation(facts, party, q_hash("q42:breached"), primary, reparation)
}

/// General dyadic / conditional obligation `O(obligation | condition)`: the obligation
/// is binding only *given* the condition holds. Returns `true` iff the conditional is
/// satisfied — either the condition does not hold (vacuously satisfied), or it holds
/// AND the obligation has been fulfilled.
///
/// Facts convention: the condition holds iff `(party, condition_pred, condition_obj)` is
/// present; the obligation is fulfilled iff `(party, q42:fulfilled, obligation_obj)` is.
/// Contrary-to-duty is the special case `condition_pred = q42:breached`. Zero-heap.
pub fn evaluate_conditional_obligation(
    facts: &[NQuin],
    party: u64,
    condition_pred: u64,
    condition_obj: u64,
    obligation_obj: u64,
) -> bool {
    let triggered = facts
        .iter()
        .any(|q| q.subject == party && q.predicate == condition_pred && q.object == condition_obj);
    if !triggered {
        return true; // condition absent → conditional obligation not triggered
    }
    let fulfilled = q_hash("q42:fulfilled");
    facts
        .iter()
        .any(|q| q.subject == party && q.predicate == fulfilled && q.object == obligation_obj)
}

// ─── Deontic lifecycle (Pending → Active → {Violated, Discharged, Defeated, Expired}) ─

/// Compute the full lifecycle status of a single norm against an effectivity window,
/// the current time, the harvested defeaters, and a fact slice.
///
/// Transition order (first match wins):
/// 1. `effective_from != 0 && now < effective_from` → [`Pending`](DeonticStatus::Pending).
/// 2. `expiry != 0 && now > expiry` → [`Expired`](DeonticStatus::Expired).
/// 3. a matching defeater → [`Defeated`](DeonticStatus::Defeated).
/// 4. in-force, then the facts decide:
///    - `OP_OBLIGATE`: `(party, q42:fulfilled, action)` → [`Discharged`]; else
///      `(party, q42:breached, action)` → [`Violated`]; else [`Active`].
///    - `OP_FORBID`: `(party, q42:performed, action)` → [`Violated`]; else [`Active`].
///    - `OP_PERMIT`: always [`Active`] (a liberty cannot be violated or discharged).
///
/// Zero-heap (linear scans). `active_defeaters` is the buffer from
/// [`harvest_defeater_fingerprints`].
pub fn norm_lifecycle_status(
    norm: &NQuin,
    now_unix: u32,
    effective_from: u32,
    active_defeaters: &[u64],
    facts: &[NQuin],
) -> DeonticStatus {
    if effective_from != 0 && now_unix < effective_from {
        return DeonticStatus::Pending;
    }
    let expiry = extract_expiry_unix32(norm.metadata);
    if expiry != 0 && now_unix > expiry {
        return DeonticStatus::Expired;
    }
    if has_defeater(active_defeaters, norm) {
        return DeonticStatus::Defeated;
    }
    let party = norm.subject;
    let action = norm.object;
    let opcode = extract_deontic_opcode(norm.predicate);
    let fact_present = |pred: u64| {
        facts
            .iter()
            .any(|q| q.subject == party && q.predicate == pred && q.object == action)
    };
    match opcode {
        OP_OBLIGATE => {
            if fact_present(q_hash("q42:fulfilled")) {
                DeonticStatus::Discharged
            } else if fact_present(q_hash("q42:breached")) {
                DeonticStatus::Violated
            } else {
                DeonticStatus::Active
            }
        }
        OP_FORBID => {
            if fact_present(q_hash("q42:performed")) {
                DeonticStatus::Violated
            } else {
                DeonticStatus::Active
            }
        }
        _ => DeonticStatus::Active, // OP_PERMIT and others: a liberty cannot be violated
    }
}

// ─── Optionality (U) and Gratuitousness (G) — derived modalities ────────────────

/// True iff action φ is **optional / indifferent** for `party`: `¬O(φ) ∧ ¬F(φ)` — no
/// active (non-defeater) obligation and no prohibition over `(party, action)` in the
/// norm slice. (An explicit `OP_OPTIONAL` assertion also counts.)
pub fn is_optional(norms: &[NQuin], party: u64, action: u64) -> bool {
    !has_active_norm(norms, party, action, OP_OBLIGATE)
        && !has_active_norm(norms, party, action, OP_FORBID)
}

/// True iff action φ is **gratuitous** (non-obligatory) for `party`: `¬O(φ)` — no
/// active obligation over `(party, action)` (it may still be permitted or forbidden).
pub fn is_gratuitous(norms: &[NQuin], party: u64, action: u64) -> bool {
    !has_active_norm(norms, party, action, OP_OBLIGATE)
}

/// Helper: is there a non-defeater norm with `opcode` binding `party` to `action`?
/// Matches the explicit modality opcode (`OP_OPTIONAL`/`OP_GRATUITOUS` short-circuit).
fn has_active_norm(norms: &[NQuin], party: u64, action: u64, opcode: u8) -> bool {
    norms.iter().any(|q| {
        q.predicate & DEFEATER_BIT == 0
            && q.subject == party
            && q.object == action
            && extract_deontic_opcode(q.predicate) == opcode
    })
}

// ─── Norm-conflict resolution (proportionality + human-rights priority) ──────────

/// Do two deontic OPCODES conflict — an obligation/permission to do φ vs a prohibition of φ?
pub fn opcodes_conflict(a: u8, b: u8) -> bool {
    let permits = |op: u8| op == OP_OBLIGATE || op == OP_PERMIT;
    (permits(a) && b == OP_FORBID) || (permits(b) && a == OP_FORBID)
}

/// Do two norms CONFLICT — same party (`subject`) over the same action (`object`), with
/// deontically opposed opcodes? Active (non-defeater) norms only.
pub fn norms_conflict(a: &NQuin, b: &NQuin) -> bool {
    a.predicate & DEFEATER_BIT == 0
        && b.predicate & DEFEATER_BIT == 0
        && a.subject == b.subject
        && a.object == b.object
        && opcodes_conflict(
            extract_deontic_opcode(a.predicate),
            extract_deontic_opcode(b.predicate),
        )
}

/// The outcome of resolving a norm conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormResolution {
    /// The first norm prevails.
    FirstPrevails,
    /// The second norm prevails.
    SecondPrevails,
    /// A genuine conflict — routed to human review (never auto-flattened).
    RequiresHumanReview,
}

/// Resolve a norm conflict by, in strict order:
///  1. **Non-derogable human-rights priority** — a norm grounded in a non-derogable instrument
///     defeats a derogable one (never weaken a non-derogable principle).
///  2. **Proportionality** — if neither/both are non-derogable, the norm whose action is
///     *proportionate* (`legal_compose::proportionality_met`: marginal harm < advantage) prevails
///     over a disproportionate one.
///  3. Otherwise **human review** — a contested norm is never auto-flattened.
///
/// `a_proportionate`/`b_proportionate` are the proportionality verdicts (`None` = unmodelled).
pub fn resolve_norm_conflict(
    a_nonderogable: bool,
    b_nonderogable: bool,
    a_proportionate: Option<bool>,
    b_proportionate: Option<bool>,
) -> NormResolution {
    match (a_nonderogable, b_nonderogable) {
        (true, false) => return NormResolution::FirstPrevails,
        (false, true) => return NormResolution::SecondPrevails,
        _ => {}
    }
    match (a_proportionate, b_proportionate) {
        (Some(true), Some(false)) => NormResolution::FirstPrevails,
        (Some(false), Some(true)) => NormResolution::SecondPrevails,
        _ => NormResolution::RequiresHumanReview,
    }
}

// ─── Permissions as non-fungible cryptographic constraints ──────────────────────

/// A collision-resistant (BLAKE3) fingerprint over a Quin's six fields — the cryptographic
/// binding anchor. Changing any field changes the fingerprint.
pub fn nquin_binding_hash(q: &NQuin) -> u64 {
    let mut bytes = [0u8; 48];
    bytes[0..8].copy_from_slice(&q.subject.to_le_bytes());
    bytes[8..16].copy_from_slice(&q.predicate.to_le_bytes());
    bytes[16..24].copy_from_slice(&q.object.to_le_bytes());
    bytes[24..32].copy_from_slice(&q.context.to_le_bytes());
    bytes[32..40].copy_from_slice(&q.metadata.to_le_bytes());
    bytes[40..48].copy_from_slice(&q.parity.to_le_bytes());
    let h = blake3::hash(&bytes);
    u64::from_le_bytes(h.as_bytes()[..8].try_into().unwrap())
}

/// Compile a permission into a **non-fungible cryptographic constraint** bound to a *specific*
/// target nquin: the constraint carries, in `context`, a BLAKE3 binding to `target`, so the
/// permission cannot be detached and reused for a different nquin — it travels persistently with
/// that exact one. The identity layer SIGNS this envelope (the engine never holds keys — see
/// `meta_deontic::endorsement_credential`); this constructs the bound, verifiable constraint.
pub fn compile_permission_constraint(action: u64, principal: u64, target: &NQuin) -> NQuin {
    let binding = nquin_binding_hash(target);
    let mut c = NQuin {
        subject: principal,
        predicate: OP_PERMIT as u64,
        object: action,
        context: binding,
        metadata: 0,
        parity: 0,
    };
    c.parity = c.subject ^ c.predicate ^ c.object ^ c.context;
    c
}

/// Verify that a permission `constraint` is bound to `target` (the non-fungibility check): its
/// `context` binding must match `target`'s current fingerprint. Tampering with `target`, or moving
/// the constraint to a different nquin, breaks the binding.
pub fn permission_binds_to(constraint: &NQuin, target: &NQuin) -> bool {
    constraint.context == nquin_binding_hash(target)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norm_conflict_detection_and_proportional_resolution() {
        let party = q_hash("did:party");
        let action = q_hash("act:disclose");
        let mk = |op: u8| {
            let mut q = NQuin {
                subject: party,
                predicate: op as u64,
                object: action,
                context: 0,
                metadata: 0,
                parity: 0,
            };
            q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
            q
        };
        // Obligate-disclose vs Forbid-disclose for the same party/action → conflict.
        assert!(norms_conflict(&mk(OP_OBLIGATE), &mk(OP_FORBID)));
        assert!(opcodes_conflict(OP_PERMIT, OP_FORBID));
        // Two obligations don't conflict; different actions don't.
        assert!(!norms_conflict(&mk(OP_OBLIGATE), &mk(OP_OBLIGATE)));
        let mut other = mk(OP_FORBID);
        other.object = q_hash("act:other");
        assert!(!norms_conflict(&mk(OP_OBLIGATE), &other));

        // Resolution: non-derogable beats derogable.
        assert_eq!(
            resolve_norm_conflict(true, false, None, None),
            NormResolution::FirstPrevails
        );
        assert_eq!(
            resolve_norm_conflict(false, true, None, None),
            NormResolution::SecondPrevails
        );
        // Neither non-derogable → proportionality decides.
        assert_eq!(
            resolve_norm_conflict(false, false, Some(true), Some(false)),
            NormResolution::FirstPrevails
        );
        assert_eq!(
            resolve_norm_conflict(false, false, Some(false), Some(true)),
            NormResolution::SecondPrevails
        );
        // Both non-derogable, or proportionality unmodelled → human review.
        assert_eq!(
            resolve_norm_conflict(true, true, None, None),
            NormResolution::RequiresHumanReview
        );
        assert_eq!(
            resolve_norm_conflict(false, false, None, None),
            NormResolution::RequiresHumanReview
        );
    }

    #[test]
    fn permission_is_non_fungibly_bound_to_its_nquin() {
        let principal = q_hash("did:principal");
        let action = q_hash("act:read");
        let mut target = NQuin {
            subject: q_hash("doc:42"),
            predicate: q_hash("q42:hasContent"),
            object: q_hash("blob:abc"),
            context: 7,
            metadata: 0,
            parity: 0,
        };
        target.parity = target.subject ^ target.predicate ^ target.object ^ target.context;

        let c = compile_permission_constraint(action, principal, &target);
        assert!(
            permission_binds_to(&c, &target),
            "the permission binds to its target nquin"
        );
        assert_eq!(extract_deontic_opcode(c.predicate), OP_PERMIT);

        // Tampering with the target breaks the binding (non-fungible / tamper-evident).
        let mut tampered = target;
        tampered.object ^= 0x1;
        assert!(
            !permission_binds_to(&c, &tampered),
            "any edit to the target breaks the binding"
        );

        // The constraint cannot be reused for a DIFFERENT nquin.
        let mut other = target;
        other.subject = q_hash("doc:99");
        other.parity = other.subject ^ other.predicate ^ other.object ^ other.context;
        assert!(
            !permission_binds_to(&c, &other),
            "permission is not fungible across nquins"
        );
    }
    use crate::q_hash;

    #[test]
    fn contrary_to_duty_requires_reparation_after_breach() {
        let party = q_hash("did:web:acme");
        let primary = q_hash("q42:protectData");
        let reparation = q_hash("q42:notifyAndRemedy");
        let mk = |s: u64, p: u64, o: u64| {
            let mut q = NQuin {
                subject: s,
                predicate: p,
                object: o,
                context: 0,
                metadata: 0,
                parity: 0,
            };
            q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
            q
        };
        // No breach → satisfied (CTD not triggered).
        assert!(evaluate_contrary_to_duty(&[], party, primary, reparation));
        // Breach without reparation → NOT satisfied.
        let breach = [mk(party, q_hash("q42:breached"), primary)];
        assert!(!evaluate_contrary_to_duty(
            &breach, party, primary, reparation
        ));
        // Breach WITH reparation → satisfied.
        let repaired = [
            mk(party, q_hash("q42:breached"), primary),
            mk(party, q_hash("q42:fulfilled"), reparation),
        ];
        assert!(evaluate_contrary_to_duty(
            &repaired, party, primary, reparation
        ));
    }

    /// Webizen values-credential smoke test (PLAN §11.3 / §17.1) — THE KEYSTONE.
    ///
    /// Proves a real values prohibition flows the live deontic lane:
    ///   N3 `Rule` (ns.webcivics.net) → `compile_n3_rule_to_norm` → `evaluate_deontic_contract`
    ///   → `DeonticVerdict`  (this is exactly what the `NativeDeonticEval` opcode dispatches to).
    /// And that the engine's NATIVE defeasibility flips Active → Defeated when a `q42:unless`
    /// defeater is present. No `n3logic.rs::infer_logic_bindings` on this path.
    #[test]
    fn values_credential_deontic_smoke() {
        use crate::modalities::logic::n3_parser::{Formula, Rule, RuleType, Term, Triple};

        // A values prohibition (UDHR Art 30 family) as a parsed-shape N3 rule:
        //   { values:Agent  values:forbids  values:DestructionOfRights }
        let prohibition = Rule {
            id: Some("UDHR-Art30-smoke"),
            rule_type: RuleType::Strict,
            weight: None,
            premise: Formula {
                triples: vec![Triple {
                    subject: Term::Uri("https://ns.webcivics.net/values/Agent"),
                    predicate: Term::Uri("https://ns.webcivics.net/values/forbids"),
                    object: Term::Uri("https://ns.webcivics.net/values/DestructionOfRights"),
                }],
            },
            conclusion: Formula { triples: vec![] },
        };
        let contract = q_hash("contract:udhr-smoke");

        // ── values rule → norm Quin (the values→deontic bridge) ──
        let norm = compile_n3_rule_to_norm(
            &crate::modalities::logic::n3_compiler::compile_rule_to_zero_heap(&prohibition),
            contract,
            0,
        )
        .expect("a values prohibition must compile to a norm Quin");
        assert_eq!(
            extract_deontic_opcode(norm.predicate),
            OP_FORBID,
            "a `values:forbids` rule must compile to an OP_FORBID norm"
        );

        // ── evaluate via the native deontic VM path → the prohibition is Active (live) ──
        let mut out = [DeonticVerdict::default(); 4];
        let n = evaluate_deontic_contract(&[norm], NOW, &mut out)
            .expect("deontic evaluation must succeed");
        assert_eq!(n, 1, "exactly one norm verdict expected");
        assert_eq!(
            out[0].status,
            DeonticStatus::Active,
            "the values prohibition holds (Active) — it is live in the engine, not bot-faked"
        );
        assert_eq!(out[0].opcode, OP_FORBID);

        // ── native defeasibility: a `q42:unless` defeater on the same party+path+contract
        //    flips Active → Defeated ("forbidden ... UNLESS lawfully authorised"). ──
        let party = q_hash("https://ns.webcivics.net/values/Agent");
        let path = q_hash("https://ns.webcivics.net/values/forbids");
        let defeater = compile_norm_quin(
            party,
            OP_PERMIT,
            path,
            q_hash("https://ns.webcivics.net/values/lawfullyAuthorised"),
            contract,
            0,
            /* is_defeater = */ true,
        );
        let mut out2 = [DeonticVerdict::default(); 4];
        let n2 = evaluate_deontic_contract(&[norm, defeater], NOW, &mut out2)
            .expect("deontic evaluation with defeater must succeed");
        assert_eq!(
            n2, 1,
            "the defeater is not a primary norm; one verdict expected"
        );
        assert_eq!(
            out2[0].status,
            DeonticStatus::Defeated,
            "an `unless` defeater on the same party+path must defeat the prohibition"
        );
    }

    fn alice() -> u64 {
        q_hash("did:web:alice.example")
    }
    fn bob() -> u64 {
        q_hash("did:web:bob.example")
    }
    fn nda() -> u64 {
        q_hash("did:web:nda:contract-001")
    }
    fn disclose_path() -> u64 {
        q_hash("q42:disclose")
    }
    fn conf_data() -> u64 {
        q_hash("q42:data:project-x:confidential")
    }

    const NOW: u32 = 1_717_200_000; // ~2024-06-01 — well before NDA expiry
    const EXPIRY_NDA: u32 = 1_830_297_600; // 2028-01-01

    fn nda_quins() -> [NQuin; 3] {
        [
            // Quin 0: Alice FORBID disclose (active)
            compile_norm_quin(
                alice(),
                OP_FORBID,
                disclose_path(),
                conf_data(),
                nda(),
                EXPIRY_NDA,
                false,
            ),
            // Quin 1: Bob FORBID disclose (active)
            compile_norm_quin(
                bob(),
                OP_FORBID,
                disclose_path(),
                conf_data(),
                nda(),
                EXPIRY_NDA,
                false,
            ),
            // Quin 2: q42:unless — Alice PERMIT disclose to auditors (defeater for Quin 0)
            compile_norm_quin(
                alice(),
                OP_PERMIT,
                disclose_path(),
                q_hash("q42:role:certified-auditor"),
                nda(),
                EXPIRY_NDA,
                true,
            ),
        ]
    }

    #[test]
    fn nda_alice_is_defeated_bob_is_active() {
        let quins = nda_quins();
        let mut out = [DeonticVerdict {
            norm: NQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            defeat_kind: DefeatKind::None,
            _pad: [0u8; 5],
        }; 8];

        let n = evaluate_deontic_contract(&quins, NOW, &mut out).unwrap();

        // Two norm Quins (Alice + Bob); defeater is not a norm.
        assert_eq!(n, 2, "expected exactly two verdicts");

        // Alice's prohibition should be defeated by Quin 2.
        let alice_verdict = out[..n].iter().find(|v| v.norm.subject == alice()).unwrap();
        assert_eq!(
            alice_verdict.status,
            DeonticStatus::Defeated,
            "Alice obligation should be defeated"
        );
        assert_eq!(alice_verdict.opcode, OP_FORBID);

        // Bob has no defeater — should be active.
        let bob_verdict = out[..n].iter().find(|v| v.norm.subject == bob()).unwrap();
        assert_eq!(
            bob_verdict.status,
            DeonticStatus::Active,
            "Bob obligation should be active"
        );
    }

    #[test]
    fn expired_norm_is_detected() {
        let past_expiry: u32 = 1_000_000; // Unix epoch far in the past
        let norm = compile_norm_quin(
            alice(),
            OP_OBLIGATE,
            disclose_path(),
            conf_data(),
            nda(),
            past_expiry,
            false,
        );
        let quins = [norm];
        let mut out = [DeonticVerdict {
            norm: NQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            defeat_kind: DefeatKind::None,
            _pad: [0u8; 5],
        }; 4];

        let n = evaluate_deontic_contract(&quins, NOW, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].status, DeonticStatus::Expired);
    }

    #[test]
    fn no_expiry_zero_is_always_valid() {
        let norm = compile_norm_quin(
            alice(),
            OP_PERMIT,
            disclose_path(),
            conf_data(),
            nda(),
            0,
            false,
        );
        let quins = [norm];
        let mut out = [DeonticVerdict {
            norm: NQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            defeat_kind: DefeatKind::None,
            _pad: [0u8; 5],
        }; 4];

        let n = evaluate_deontic_contract(&quins, u32::MAX, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            out[0].status,
            DeonticStatus::Active,
            "zero expiry should never expire"
        );
    }

    #[test]
    fn non_deontic_quins_are_skipped() {
        // Plain SHACL/data Quin with opcode 0x00 — should produce no verdicts.
        let plain = NQuin {
            subject: 1,
            predicate: 0x00,
            object: 2,
            context: 3,
            metadata: 0,
            parity: 0,
        };
        let mut out = [DeonticVerdict {
            norm: NQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            defeat_kind: DefeatKind::None,
            _pad: [0u8; 5],
        }; 4];

        let n = evaluate_deontic_contract(&[plain], NOW, &mut out).unwrap();
        assert_eq!(n, 0, "non-deontic Quins must be silently skipped");
    }

    #[test]
    fn output_buffer_full_returns_error() {
        let quins = nda_quins(); // 2 norm Quins
        let mut out = [DeonticVerdict {
            norm: NQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            defeat_kind: DefeatKind::None,
            _pad: [0u8; 5],
        }; 1]; // one slot — too small

        assert_eq!(
            evaluate_deontic_contract(&quins, NOW, &mut out),
            Err(DeonticError::OutputBufferFull)
        );
    }

    #[test]
    fn empty_slice_returns_zero_verdicts() {
        let mut out = [DeonticVerdict {
            norm: NQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            defeat_kind: DefeatKind::None,
            _pad: [0u8; 5],
        }; 4];
        let n = evaluate_deontic_contract(&[], NOW, &mut out).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn guardianship_contract_temporal_expiry() {
        let guardian = q_hash("did:web:guardian.example");
        let ward = q_hash("did:web:ward.example");
        let contract = q_hash("did:web:guardianship:contract-002");
        let path = q_hash("q42:actInBestInterest");
        let majority_epoch: u32 = 1_893_456_000; // 2030-01-01

        let obligation = compile_norm_quin(
            guardian,
            OP_OBLIGATE,
            path,
            ward,
            contract,
            majority_epoch,
            false,
        );
        let quins = [obligation];

        let mut out = [DeonticVerdict {
            norm: NQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            defeat_kind: DefeatKind::None,
            _pad: [0u8; 5],
        }; 4];

        // Before majority — obligation is active.
        let n = evaluate_deontic_contract(&quins, NOW, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].status, DeonticStatus::Active);

        // After majority — obligation has expired.
        let n = evaluate_deontic_contract(&quins, majority_epoch + 1, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].status, DeonticStatus::Expired);
    }

    #[test]
    fn opcode_constants_are_distinct_from_mini_parser_range() {
        // mini_parser uses 0x00–0x04; deontic opcodes must not collide.
        assert!(OP_OBLIGATE > 0x04);
        assert!(OP_PERMIT > 0x04);
        assert!(OP_FORBID > 0x04);
        assert_ne!(OP_OBLIGATE, OP_PERMIT);
        assert_ne!(OP_PERMIT, OP_FORBID);
        assert_ne!(OP_OBLIGATE, OP_FORBID);
    }

    #[test]
    fn defeater_bit_is_msb() {
        assert_eq!(DEFEATER_BIT, 1u64 << 63);
    }

    #[test]
    fn compile_norm_quin_parity_is_xor_fold() {
        let q = compile_norm_quin(
            alice(),
            OP_FORBID,
            disclose_path(),
            conf_data(),
            nda(),
            EXPIRY_NDA,
            false,
        );
        let expected = q.subject ^ q.predicate ^ q.object ^ q.context;
        assert_eq!(
            q.parity, expected,
            "parity must be XOR fold of semantic fields"
        );
    }

    #[test]
    fn compile_n3_defeater_sets_defeater_bit() {
        use crate::modalities::logic::n3_parser::{Formula, Rule, RuleType, Triple};
        let rule = Rule {
            id: None,
            rule_type: RuleType::Defeater,
            weight: None,
            premise: Formula {
                triples: vec![Triple {
                    subject: Term::Uri("did:web:alice.example".into()),
                    predicate: Term::Uri("q42:disclose".into()),
                    object: Term::Uri("q42:role:certified-auditor".into()),
                }],
            },
            conclusion: Formula {
                triples: vec![Triple {
                    subject: Term::Uri("did:web:alice.example".into()),
                    predicate: Term::Uri("q42:disclose".into()),
                    object: Term::Uri("true".into()),
                }],
            },
        };
        let q = compile_n3_rule_to_norm(
            &crate::modalities::logic::n3_compiler::compile_rule_to_zero_heap(&rule),
            nda(),
            EXPIRY_NDA,
        )
        .unwrap();
        assert_ne!(q.predicate & DEFEATER_BIT, 0);
    }

    #[test]
    fn compile_n3_defeasible_permit_rule() {
        use crate::modalities::logic::n3_parser::{Formula, Rule, RuleType, Triple};
        let rule = Rule {
            id: None,
            rule_type: RuleType::Defeasible,
            weight: None,
            premise: Formula {
                triples: vec![Triple {
                    subject: Term::Uri("did:web:bob.example".into()),
                    predicate: Term::Uri("q42:permitAccess".into()),
                    object: Term::Uri("q42:data:project-x".into()),
                }],
            },
            conclusion: Formula { triples: vec![] },
        };
        let q = compile_n3_rule_to_norm(
            &crate::modalities::logic::n3_compiler::compile_rule_to_zero_heap(&rule),
            nda(),
            0,
        )
        .unwrap();
        assert_eq!(extract_deontic_opcode(q.predicate), OP_PERMIT);
        assert_eq!(q.predicate & DEFEATER_BIT, 0);
    }

    #[test]
    fn compile_n3_malformed_rule_returns_none() {
        use crate::modalities::logic::n3_parser::{Formula, Rule, RuleType};
        let rule = Rule {
            id: None,
            rule_type: RuleType::Strict,
            weight: None,
            premise: Formula { triples: vec![] },
            conclusion: Formula { triples: vec![] },
        };
        assert!(compile_n3_rule_to_norm(
            &crate::modalities::logic::n3_compiler::compile_rule_to_zero_heap(&rule),
            nda(),
            0
        )
        .is_none());
    }

    // ─── Phase 1: SDL⁺ extensions (DEONTIC_LOGIC_PLAN §4) ───────────────────────

    fn mkfact(s: u64, p: u64, o: u64) -> NQuin {
        let mut q = NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn lifecycle_pending_active_discharged_violated() {
        let party = alice();
        let action = conf_data();
        let duty = compile_norm_quin(party, OP_OBLIGATE, disclose_path(), action, nda(), 0, false);

        // effective_from in the future → Pending.
        assert_eq!(
            norm_lifecycle_status(&duty, NOW, NOW + 1000, &[], &[]),
            DeonticStatus::Pending
        );
        // in force, no facts → Active.
        assert_eq!(
            norm_lifecycle_status(&duty, NOW, 0, &[], &[]),
            DeonticStatus::Active
        );
        // fulfilled fact → Discharged.
        let fulfilled = [mkfact(party, q_hash("q42:fulfilled"), action)];
        assert_eq!(
            norm_lifecycle_status(&duty, NOW, 0, &[], &fulfilled),
            DeonticStatus::Discharged
        );
        // breached fact → Violated.
        let breached = [mkfact(party, q_hash("q42:breached"), action)];
        assert_eq!(
            norm_lifecycle_status(&duty, NOW, 0, &[], &breached),
            DeonticStatus::Violated
        );
    }

    #[test]
    fn lifecycle_forbid_violated_by_performance() {
        let party = bob();
        let action = conf_data();
        let prohibition =
            compile_norm_quin(party, OP_FORBID, disclose_path(), action, nda(), 0, false);
        let performed = [mkfact(party, q_hash("q42:performed"), action)];
        assert_eq!(
            norm_lifecycle_status(&prohibition, NOW, 0, &[], &performed),
            DeonticStatus::Violated
        );
        assert_eq!(
            norm_lifecycle_status(&prohibition, NOW, 0, &[], &[]),
            DeonticStatus::Active
        );
    }

    #[test]
    fn lifecycle_expiry_and_defeater_precedence() {
        let party = alice();
        let action = conf_data();
        let duty = compile_norm_quin(
            party,
            OP_OBLIGATE,
            disclose_path(),
            action,
            nda(),
            EXPIRY_NDA,
            false,
        );
        let fulfilled = [mkfact(party, q_hash("q42:fulfilled"), action)];
        // past expiry → Expired (temporal precedes facts).
        assert_eq!(
            norm_lifecycle_status(&duty, EXPIRY_NDA + 1, 0, &[], &fulfilled),
            DeonticStatus::Expired
        );
        // matching defeater → Defeated (precedes facts).
        let df = defeater_fingerprint(&duty);
        assert_eq!(
            norm_lifecycle_status(&duty, NOW, 0, &[df], &fulfilled),
            DeonticStatus::Defeated
        );
    }

    #[test]
    fn optionality_and_gratuitousness() {
        let party = alice();
        let action = q_hash("q42:donate");
        // no norms → optional and gratuitous.
        assert!(is_optional(&[], party, action));
        assert!(is_gratuitous(&[], party, action));
        // obligation → neither.
        let oblig = compile_norm_quin(party, OP_OBLIGATE, disclose_path(), action, nda(), 0, false);
        assert!(!is_optional(&[oblig], party, action));
        assert!(!is_gratuitous(&[oblig], party, action));
        // permission alone → optional and gratuitous.
        let perm = compile_norm_quin(party, OP_PERMIT, disclose_path(), action, nda(), 0, false);
        assert!(is_optional(&[perm], party, action));
        assert!(is_gratuitous(&[perm], party, action));
        // prohibition → gratuitous (not obliged) but NOT optional (forbidden).
        let forbid = compile_norm_quin(party, OP_FORBID, disclose_path(), action, nda(), 0, false);
        assert!(!is_optional(&[forbid], party, action));
        assert!(is_gratuitous(&[forbid], party, action));
    }

    #[test]
    fn undercutting_vs_rebutting_defeater_kind() {
        let party = alice();
        let action = conf_data();
        let duty = compile_norm_quin(party, OP_OBLIGATE, disclose_path(), action, nda(), 0, false);
        let mut out = [DeonticVerdict::default(); 4];

        // Rebutting: DEFEATER_BIT + a contrary opcode (PERMIT) on the same path.
        let rebut = compile_norm_quin(
            party,
            OP_PERMIT,
            disclose_path(),
            q_hash("q42:exc"),
            nda(),
            0,
            true,
        );
        let n = evaluate_deontic_contract(&[duty, rebut], NOW, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].status, DeonticStatus::Defeated);
        assert_eq!(out[0].defeat_kind, DefeatKind::Rebutting);

        // Undercutting: DEFEATER_BIT + OP_UNDERCUT on the same path → link-invalidation.
        let undercut = compile_norm_quin(
            party,
            OP_UNDERCUT,
            disclose_path(),
            q_hash("q42:exc"),
            nda(),
            0,
            true,
        );
        let n = evaluate_deontic_contract(&[duty, undercut], NOW, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].status, DeonticStatus::Defeated);
        assert_eq!(out[0].defeat_kind, DefeatKind::Undercutting);
    }

    #[test]
    fn dyadic_conditional_obligation() {
        let party = alice();
        let condition = q_hash("q42:dataCollected");
        let obligation = q_hash("q42:obtainConsent");
        let cond_pred = q_hash("q42:holds");
        // condition absent → vacuously satisfied.
        assert!(evaluate_conditional_obligation(
            &[],
            party,
            cond_pred,
            condition,
            obligation
        ));
        // condition present, unfulfilled → not satisfied.
        let triggered = [mkfact(party, cond_pred, condition)];
        assert!(!evaluate_conditional_obligation(
            &triggered, party, cond_pred, condition, obligation
        ));
        // condition present, fulfilled → satisfied.
        let done = [
            mkfact(party, cond_pred, condition),
            mkfact(party, q_hash("q42:fulfilled"), obligation),
        ];
        assert!(evaluate_conditional_obligation(
            &done, party, cond_pred, condition, obligation
        ));
    }
}
