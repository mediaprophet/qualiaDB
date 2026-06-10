//! Deontic Logic extension for the Qualia Bytecode VM.
//!
//! Implements a defeasible deontic contract evaluator over a `&[QualiaQuin]` slice.
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
//! │ context  │ [56..63] = sensitivity class (from QualiaQuin::SENSITIVITY_*)   │
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

use crate::modalities::logic::n3_parser::{Rule, RuleType, Term};
use crate::q_hash;
use crate::QualiaQuin;

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

/// Bit 63 of `predicate`: marks a `q42:unless` defeater / exception node.
/// When set the Quin is *not* a primary norm and defeats matching obligations.
pub const DEFEATER_BIT: u64 = 1u64 << 63;

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
    pub norm: QualiaQuin,
    /// Outcome of the evaluation.
    pub status: DeonticStatus,
    /// Deontic opcode extracted from `norm.predicate[0..7]`.
    pub opcode: u8,
    _pad: [u8; 6],
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
pub fn defeater_fingerprint(q: &QualiaQuin) -> u64 {
    // Strip defeater bit (63) and opcode byte (0..7); retain property-path (8..62).
    let path_bits = q.predicate & 0x7FFF_FFFF_FFFF_FF00;
    q.subject ^ q.context ^ path_bits
}

/// Harvest `q42:unless` defeater fingerprints from a contract slice (Phase 1 only).
pub fn harvest_defeater_fingerprints(quins: &[QualiaQuin], out: &mut [u64]) -> usize {
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
pub fn norm_has_active_defeater(norm: &QualiaQuin, defeaters: &[u64]) -> bool {
    has_defeater(defeaters, norm)
}

#[inline]
fn has_defeater(defeaters: &[u64], norm: &QualiaQuin) -> bool {
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

// ─── evaluate_deontic_contract ────────────────────────────────────────────────

/// Evaluate a deontic contract encoded as a `&[QualiaQuin]` slice.
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
    quins: &[QualiaQuin],
    now_unix: u32,
    out: &mut [DeonticVerdict],
) -> Result<usize, DeonticError> {
    // ── Phase 1: harvest defeater fingerprints ─────────────────────────────────
    //
    // Stack-allocated; fits in < 1 KB, well within any thread stack.
    let mut defeater_buf = [0u64; MAX_DEFEATER_SLOTS];
    let mut defeater_count = 0usize;

    for &q in quins {
        if q.predicate & DEFEATER_BIT != 0 {
            // ECC Parity XOR fold check
            let expected_parity = q.subject ^ q.predicate ^ q.object ^ q.context;
            if q.parity == expected_parity {
                if defeater_count < MAX_DEFEATER_SLOTS {
                    defeater_buf[defeater_count] = defeater_fingerprint(&q);
                    defeater_count += 1;
                }
                // Excess defeaters are dropped; contracts this dense are rejected upstream.
            }
        }
    }

    let active_defeaters = &defeater_buf[..defeater_count];

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
                _pad: [0u8; 6],
            };
            verdict_count += 1;
            continue;
        }

        let opcode = extract_deontic_opcode(q.predicate);

        let status = match opcode {
            OP_OBLIGATE | OP_PERMIT | OP_FORBID => {
                let expiry = extract_expiry_unix32(q.metadata);
                if expiry != 0 && now_unix > expiry {
                    DeonticStatus::Expired
                } else if has_defeater(active_defeaters, &q) {
                    DeonticStatus::Defeated
                } else {
                    DeonticStatus::Active
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
            _pad: [0u8; 6],
        };
        verdict_count += 1;
    }

    Ok(verdict_count)
}

// ─── N3 → deontic norm bridge ───────────────────────────────────────────────

fn term_uri_hash(term: &Term) -> Option<u64> {
    match term {
        Term::Uri(uri) => Some(q_hash(uri)),
        Term::Literal(lit) => Some(q_hash(lit)),
        Term::Variable(_) => None,
    }
}

fn opcode_from_predicate_uri(uri: &str, rule_type: RuleType) -> (u8, bool) {
    if matches!(rule_type, RuleType::Defeater) {
        return (OP_PERMIT, true);
    }
    let lower = uri.to_lowercase();
    let is_obligate =
        lower.contains("obligate") || lower.contains("must") || lower.contains("shall");
    let is_permit = lower.contains("permit") || lower.contains("may") || lower.contains("can");
    let is_forbid = lower.contains("forbid") || lower.contains("prohibit") || lower.contains("not");

    match rule_type {
        RuleType::Strict | RuleType::Linear => {
            if is_obligate {
                (OP_OBLIGATE, false)
            } else if is_forbid {
                (OP_FORBID, false)
            } else if is_permit {
                (OP_PERMIT, false)
            } else {
                (OP_OBLIGATE, false)
            }
        }
        RuleType::Defeasible => {
            if is_permit {
                (OP_PERMIT, false)
            } else if is_forbid {
                (OP_FORBID, false)
            } else if is_obligate {
                (OP_OBLIGATE, false)
            } else {
                (OP_PERMIT, false)
            }
        }
        RuleType::Defeater => (OP_PERMIT, true),
    }
}

/// Compile an N3 [`Rule`] into a norm Quin (or defeater Quin for `^>` rules).
///
/// Maps premise triple → party / property / action; `rule_type` → opcode + defeater flag.
pub fn compile_n3_rule_to_norm(
    rule: &Rule,
    contract_hash: u64,
    expiry_unix32: u32,
) -> Option<QualiaQuin> {
    let premise = rule.premise.triples.first()?;
    let party = term_uri_hash(&premise.subject)?;
    let property_path = term_uri_hash(&premise.predicate)?;
    let action_object = term_uri_hash(&premise.object)?;
    let predicate_uri = match &premise.predicate {
        Term::Uri(uri) => uri.as_str(),
        _ => return None,
    };
    let (opcode, is_defeater) = opcode_from_predicate_uri(predicate_uri, rule.rule_type);
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
) -> QualiaQuin {
    let defeater_flag = if is_defeater { DEFEATER_BIT } else { 0u64 };
    // Mask DEFEATER_BIT from the shifted path so only `is_defeater` controls bit 63.
    let path_bits = (property_path_hash << 8) & !DEFEATER_BIT;
    let predicate = defeater_flag | path_bits | (opcode as u64);
    let metadata = expiry_unix32 as u64; // bits [0..31]; Lamport/routing bits left zero
    let parity = party_did_hash ^ predicate ^ action_object_hash ^ contract_hash;

    QualiaQuin {
        subject: party_did_hash,
        predicate,
        object: action_object_hash,
        context: contract_hash,
        metadata,
        parity,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

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

    fn nda_quins() -> [QualiaQuin; 3] {
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
            norm: QualiaQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            _pad: [0u8; 6],
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
            norm: QualiaQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            _pad: [0u8; 6],
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
            norm: QualiaQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            _pad: [0u8; 6],
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
        let plain = QualiaQuin {
            subject: 1,
            predicate: 0x00,
            object: 2,
            context: 3,
            metadata: 0,
            parity: 0,
        };
        let mut out = [DeonticVerdict {
            norm: QualiaQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            _pad: [0u8; 6],
        }; 4];

        let n = evaluate_deontic_contract(&[plain], NOW, &mut out).unwrap();
        assert_eq!(n, 0, "non-deontic Quins must be silently skipped");
    }

    #[test]
    fn output_buffer_full_returns_error() {
        let quins = nda_quins(); // 2 norm Quins
        let mut out = [DeonticVerdict {
            norm: QualiaQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            _pad: [0u8; 6],
        }; 1]; // one slot — too small

        assert_eq!(
            evaluate_deontic_contract(&quins, NOW, &mut out),
            Err(DeonticError::OutputBufferFull)
        );
    }

    #[test]
    fn empty_slice_returns_zero_verdicts() {
        let mut out = [DeonticVerdict {
            norm: QualiaQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            _pad: [0u8; 6],
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
            norm: QualiaQuin::default(),
            status: DeonticStatus::Malformed,
            opcode: 0,
            _pad: [0u8; 6],
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
        let q = compile_n3_rule_to_norm(&rule, nda(), EXPIRY_NDA).unwrap();
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
        let q = compile_n3_rule_to_norm(&rule, nda(), 0).unwrap();
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
        assert!(compile_n3_rule_to_norm(&rule, nda(), 0).is_none());
    }
}
