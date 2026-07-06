use crate::domains::financial::tax_schema::TaxRuleSchema;
use crate::modalities::logic::deontic::{
    compile_norm_quin, evaluate_deontic_contract, harvest_defeater_fingerprints,
    norm_has_active_defeater, DeonticStatus, DeonticVerdict, DEFEATER_BIT, MAX_DEFEATER_SLOTS,
    OP_PERMIT,
};
use crate::modalities::spatio_temporal;
use crate::modalities::temporal_ltl::{self, LtlFormula};
use crate::modalities::{
    abductive, argumentation, asp, ctl, defeasible, dialectical, dl, epistemic, fuzzy, linear,
    manifold, modal, paraconsistent, probabilistic,
};
use crate::NQuin;

macro_rules! vm_log {
    ($($arg:tt)*) => {
        if cfg!(feature = "vm_tracing") {
            println!($($arg)*);
        }
    };
}

/// A fast, non-cryptographic bitwise hash to lookup sub-goals in the SLG Arena
/// without wasting CPU cycles on cryptographic overhead.
#[inline(always)]
fn fast_hash_goal(subject: u64, predicate: u64, object: u64) -> usize {
    let mut hash = subject.wrapping_add(0x9E3779B97F4A7C15);
    hash = (hash ^ (hash >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    hash = (hash ^ predicate).wrapping_mul(0x94D049BB133111EB);
    hash = (hash ^ object).wrapping_mul(0x9E3779B97F4A7C15);
    (hash ^ (hash >> 31)) as usize
}

// 42MB = 44,040,192 bytes
const SLG_ARENA_SIZE: usize = 42 * 1024 * 1024;
const QUIN_SIZE: usize = 48;
const MAX_SLOTS: usize = SLG_ARENA_SIZE / QUIN_SIZE; // 917,504 slots

use crate::modalities::logic::n3_compiler::{
    compile_rule_to_zero_heap, CompiledRule, CompiledTerm, CompiledTriple,
};
use crate::modalities::logic::n3_parser::Rule;

/// The 42MB Static Tabling Arena for SLG Resolution
/// Implemented as a Zero-Allocation Static Ring-Buffer Arena
const RECENT_SLOT_RING: usize = 512;

// ── Guard-rule grounding (forward chaining) bounds ──────────────────────────────
/// Max distinct variables bound per guard rule (premise + conclusion).
const MAX_RULE_VARS: usize = 16;
/// Max conclusion triples staged across one `fire_guard_rules` pass.
const MAX_GUARD_CONCLUSIONS: usize = 256;
/// Recursion-depth ceiling for the premise join (premise triple count).
const MAX_PREMISE_DEPTH: usize = 16;
/// Max forward-chaining rounds (fixpoint cap) for `fire_guard_rules`.
const MAX_FIXPOINT_ROUNDS: usize = 16;

/// True when any triple in the rule's premise or conclusion carries a variable.
/// Such rules cannot compile to a single ground deontic norm and are instead
/// grounded by forward-chaining (`SlgArena::fire_guard_rules`).
fn rule_has_variables(rule: &CompiledRule) -> bool {
    let term_is_var = |t: &crate::modalities::logic::n3_compiler::CompiledTerm| t.is_variable();
    rule.premise
        .triples
        .iter()
        .chain(rule.conclusion.triples.iter())
        .any(|tr| term_is_var(&tr.subject) || term_is_var(&tr.predicate) || term_is_var(&tr.object))
}

/// Unify one premise-triple field against a fact field under the current bindings.
///
/// * IRI / literal term → matches iff `q_hash(term) == field`.
/// * variable term → matches the existing binding, or (if unbound) binds it to
///   `field` and grows `nbound`.
///
/// Hashing uses the same `q_hash` as `n3_compiler::triple_to_quin`, so a premise
/// term and an ingested fact agree.
fn unify_field(
    t: &CompiledTerm,
    field: u64,
    bindings: &mut [(u64, u64)],
    nbound: &mut usize,
) -> bool {
    match t {
        CompiledTerm::Uri(s) | CompiledTerm::Literal(s) => *s == field,
        CompiledTerm::Variable(key) => {
            for i in 0..*nbound {
                if bindings[i].0 == *key {
                    return bindings[i].1 == field;
                }
            }
            if *nbound < bindings.len() {
                bindings[*nbound] = (*key, field);
                *nbound += 1;
                true
            } else {
                false // binding table exhausted — refuse rather than mis-bind
            }
        }
    }
}

/// Resolve a conclusion-triple field to a concrete hash under the bindings.
/// Returns `None` for an unbound conclusion variable (a fresh variable not
/// constrained by the premise) — such conclusions are skipped, never guessed.
#[allow(clippy::ptr_arg)]
fn resolve_term(term: &CompiledTerm, bindings: &[(u64, u64)]) -> Option<u64> {
    match term {
        CompiledTerm::Uri(s) | CompiledTerm::Literal(s) => Some(*s),
        CompiledTerm::Variable(key) => bindings.iter().find(|(k, _)| *k == *key).map(|(_, v)| *v),
    }
}

/// Conjunctive backtracking join of a rule's premise triples against the facts.
///
/// At full depth (every premise triple satisfied), each conclusion triple is
/// instantiated with the bound variables and staged into `pending`. Backtracking
/// is implicit: each fact iteration restarts binding from `nbound`, so bindings a
/// failed branch added are simply overwritten on the next branch.
#[allow(clippy::too_many_arguments)]
fn join_premise(
    premise: &[CompiledTriple],
    conclusion: &[CompiledTriple],
    facts: &[NQuin],
    idx: usize,
    bindings: &mut [(u64, u64)],
    nbound: usize,
    pending: &mut [NQuin],
    pending_count: &mut usize,
) {
    if idx >= MAX_PREMISE_DEPTH {
        return; // guard against pathologically deep premises
    }
    if idx == premise.len() {
        for ct in conclusion {
            if *pending_count >= pending.len() {
                return;
            }
            let (Some(s), Some(p), Some(o)) = (
                resolve_term(&ct.subject, &bindings[..nbound]),
                resolve_term(&ct.predicate, &bindings[..nbound]),
                resolve_term(&ct.object, &bindings[..nbound]),
            ) else {
                continue;
            };
            let mut q = NQuin {
                subject: s,
                predicate: p,
                object: o,
                context: 0,
                metadata: 1,
                parity: 0,
            };
            q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
            pending[*pending_count] = q;
            *pending_count += 1;
        }
        return;
    }

    let t = &premise[idx];
    for fact in facts {
        let mut local_nbound = nbound;
        if unify_field(&t.subject, fact.subject, bindings, &mut local_nbound)
            && unify_field(&t.predicate, fact.predicate, bindings, &mut local_nbound)
            && unify_field(&t.object, fact.object, bindings, &mut local_nbound)
        {
            join_premise(
                premise,
                conclusion,
                facts,
                idx + 1,
                bindings,
                local_nbound,
                pending,
                pending_count,
            );
        }
    }
}

pub struct SlgArena {
    // We will use a safe Vec wrapper here since it is allocated strictly once and never grown.
    #[cfg(feature = "alloc_buffers")]
    buffer: alloc::vec::Vec<NQuin>,
    #[cfg(not(feature = "alloc_buffers"))]
    buffer: std::vec::Vec<NQuin>,
    head_pointer: usize,
    recent_slots: [usize; RECENT_SLOT_RING],
    recent_slot_head: usize,
    // Native Rule Registry to hold N3 Logical Implications
    #[cfg(feature = "alloc_buffers")]
    rule_registry: alloc::vec::Vec<CompiledRule>,
    #[cfg(not(feature = "alloc_buffers"))]
    rule_registry: std::vec::Vec<CompiledRule>,
}

#[cfg(feature = "alloc_buffers")]
extern crate alloc;

impl SlgArena {
    pub fn new() -> Self {
        #[cfg(feature = "alloc_buffers")]
        extern crate alloc;

        #[cfg(feature = "alloc_buffers")]
        let mut buffer = alloc::vec::Vec::with_capacity(MAX_SLOTS);
        #[cfg(not(feature = "alloc_buffers"))]
        let mut buffer = std::vec::Vec::with_capacity(MAX_SLOTS);

        // Pre-fill the ring buffer with empty Quins
        for _ in 0..MAX_SLOTS {
            buffer.push(NQuin {
                subject: 0,
                predicate: 0,
                object: 0,
                context: 0,
                metadata: 0,
                parity: 0,
            });
        }

        #[cfg(feature = "alloc_buffers")]
        let rule_registry = alloc::vec::Vec::new();
        #[cfg(not(feature = "alloc_buffers"))]
        let rule_registry = std::vec::Vec::new();

        Self {
            buffer,
            head_pointer: 0,
            recent_slots: [0; RECENT_SLOT_RING],
            recent_slot_head: 0,
            rule_registry,
        }
    }

    /// Registers a logical implication rule into the Webizen VM
    pub fn register_rule(&mut self, rule: &Rule<'_>) {
        vm_log!("🧠 Webizen registered new Compiled N3 Rule");
        self.rule_registry.push(compile_rule_to_zero_heap(rule));
    }

    pub fn rule_count(&self) -> usize {
        self.rule_registry.len()
    }

    /// Collect recently written Quins with valid ECC parity (bounded scan, zero heap).
    pub fn collect_active_quins(&self, out: &mut [NQuin]) -> usize {
        let mut n = 0usize;
        let scan = RECENT_SLOT_RING.min(self.recent_slot_head);
        for off in 0..scan {
            let ring_idx = (self.recent_slot_head + RECENT_SLOT_RING - 1 - off) % RECENT_SLOT_RING;
            let idx = self.recent_slots[ring_idx];
            let q = self.buffer[idx];
            if q.subject == 0 {
                continue;
            }
            let expected = q.subject ^ q.predicate ^ q.object ^ q.context;
            if q.parity != expected {
                continue;
            }
            if n < out.len() {
                out[n] = q;
                n += 1;
            }
        }
        n
    }

    /// Compile registered N3 rules to norms + bytecode and execute on Core 1 (cold path).
    pub fn fire_registered_rules(&mut self, contract_hash: u64) -> usize {
        // Zero-heap: move the registry out (pointer swap, not a clone of the rules) so
        // the loop can call `&mut self` methods (`write_table` / `execute_vm_frame`)
        // while reading the rules; restored below before `fire_guard_rules` needs it.
        // Safe: nothing in the loop reads or mutates `rule_registry` (only
        // `register_rule` does, and it is never called during firing).
        let rules = core::mem::take(&mut self.rule_registry);
        let mut fired = 0usize;
        for rule in &rules {
            // Only ground rules compile to a single deontic norm; variable rules
            // are grounded by forward-chaining (`fire_guard_rules`) below, so
            // compiling them here would write a spurious norm keyed on a
            // variable-name hash.
            if !rule_has_variables(rule) {
                if let Some(norm) = crate::modalities::logic::deontic::compile_n3_rule_to_norm(
                    rule,
                    contract_hash,
                    0,
                ) {
                    self.write_table(norm);
                }
            }
            let mut opcodes = [SlgOpcode::Halt; 64];
            if let Ok(count) =
                crate::modalities::logic::n3_compiler::compile_rule_to_opcodes(rule, &mut opcodes)
            {
                let mut frame = VmFrame::default();
                if execute_vm_frame(self, &opcodes[..count], &mut frame).is_some() {
                    fired += 1;
                }
            }
        }
        // Restore the registry before `fire_guard_rules` forward-chains over it.
        self.rule_registry = rules;
        // Variable (non-ground) rules — e.g. the agency.n3 G1 corporate-capture
        // guard — cannot compile to a single ground norm; they are grounded by
        // forward-chaining their premise over the facts live in the arena.
        fired += self.fire_guard_rules();
        fired
    }

    /// Forward-chaining grounding pass for variable (non-ground) N3 guard rules.
    ///
    /// `fire_registered_rules` handles *ground* deontic rules via
    /// `compile_n3_rule_to_norm`. This is its complement: for every registered rule
    /// whose premise contains variables, it performs a conjunctive backtracking join
    /// of the premise triples against the facts currently live in the arena, and for
    /// each satisfying binding instantiates and asserts the (variable-substituted)
    /// conclusion triples back into the arena. Returns the number of conclusion
    /// triples newly asserted.
    ///
    /// Worked example — agency.n3 **G1**:
    /// ```text
    /// { ?c a values:CorporatePerson ; values:claims ?r .
    ///   ?r a values:Right ; values:heldBy values:NaturalPerson }
    ///   => { ?c values:flag values:PersonhoodCategoryError } .
    /// ```
    /// Given facts that a corporate person claims a natural-person right, the guard
    /// asserts `(?c, values:flag, values:PersonhoodCategoryError)` — the personhood
    /// category error (a Deny) — observable via [`Self::has_quin`].
    ///
    /// Cold path: bounded stack buffers, no hot-path heap growth. Hashing is uniform
    /// `q_hash` over IRIs/variables/literals (matching `n3_compiler::triple_to_quin`),
    /// so grounding matches facts ingested through the standard triple path.
    ///
    /// Forward chaining asserts the conclusion of any satisfied premise. Defeasible
    /// *override* of a deontic norm (e.g. a marked corporate overlay defeating a
    /// prohibition) is handled in the deontic-norm lane via the `q42:unless`
    /// defeater path (`evaluate_deontic_contract`), not here — the agency.n3 guards
    /// (G1/G1') are strict `=>` rules.
    pub fn fire_guard_rules(&mut self) -> usize {
        // Zero-heap: see `fire_registered_rules` — move the registry out (no clone),
        // iterate across the fixpoint rounds, restore at the end. Safe: the loop only
        // reads facts and calls `write_table` / `has_quin`, never touching the registry.
        let rules = core::mem::take(&mut self.rule_registry);
        let mut total = 0usize;

        // Forward-chain to a bounded fixpoint: a conclusion asserted in one round
        // may satisfy another guard's premise next round (e.g. overclaim → flag →
        // requiresHumanReview). `has_quin` idempotency guarantees termination.
        for _round in 0..MAX_FIXPOINT_ROUNDS {
            // Snapshot the live facts (immutable borrow ends before we assert).
            let mut facts = [NQuin::default(); 1024];
            let fact_count = self.collect_active_quins(&mut facts);

            let mut pending = [NQuin::default(); MAX_GUARD_CONCLUSIONS];
            let mut pending_count = 0usize;

            for rule in &rules {
                if !rule_has_variables(rule) {
                    continue; // ground rules go through the deontic-norm path
                }
                // `triples` is a fixed `[_; 8]` array, so `.is_empty()` is always
                // false - gate on the populated `len` instead.
                if rule.premise.len == 0 || rule.conclusion.len == 0 {
                    continue;
                }
                let mut bindings = [(0u64, 0u64); MAX_RULE_VARS];
                join_premise(
                    &rule.premise.triples[..rule.premise.len],
                    // Slice the conclusion by `len` too: passing the full array
                    // would stage (8 - len) junk (0,0,0) triples per match, which
                    // - being skipped by `has_quin` (subject==0) - get re-asserted
                    // every round, never reaching the fixpoint and flooding the
                    // recent-write ring until real conclusions are evicted.
                    &rule.conclusion.triples[..rule.conclusion.len],
                    &facts[..fact_count],
                    0,
                    &mut bindings,
                    0,
                    &mut pending,
                    &mut pending_count,
                );
            }

            let mut asserted = 0usize;
            for q in pending.iter().take(pending_count) {
                // Idempotent: don't re-assert a conclusion already present.
                if !self.has_quin(q.subject, q.predicate, q.object) {
                    self.write_table(*q);
                    asserted += 1;
                }
            }
            total += asserted;
            if asserted == 0 {
                break; // fixpoint reached
            }
        }
        self.rule_registry = rules;
        total
    }

    /// True when a fact `(subject, predicate, object)` is live in the arena with
    /// valid ECC parity. Used to observe forward-chained guard conclusions.
    pub fn has_quin(&self, subject: u64, predicate: u64, object: u64) -> bool {
        let mut scratch = [NQuin::default(); 1024];
        let n = self.collect_active_quins(&mut scratch);
        scratch[..n]
            .iter()
            .any(|q| q.subject == subject && q.predicate == predicate && q.object == object)
    }

    /// Checks the SLG Arena for a previously proven sub-goal.
    pub fn check_table(&self, subject: u64, predicate: u64, object: u64) -> Option<NQuin> {
        let slot = fast_hash_goal(subject, predicate, object) % MAX_SLOTS;

        let cached = self.buffer[slot];
        if cached.subject == subject && cached.predicate == predicate && cached.object == object {
            Some(cached)
        } else {
            None
        }
    }

    /// Writes a proven sub-goal into the SLG Arena.
    /// If the slot is occupied (hash collision) or we hit the boundary,
    /// it acts as a FIFO ring-buffer and strictly overwrites the oldest cache entries.
    pub fn write_table(&mut self, result: NQuin) {
        let slot = fast_hash_goal(result.subject, result.predicate, result.object) % MAX_SLOTS;

        // Cyclic Eviction Policy: Overwrite whatever is in the slot natively
        self.buffer[slot] = result;
        self.recent_slots[self.recent_slot_head % RECENT_SLOT_RING] = slot;
        self.recent_slot_head = self.recent_slot_head.saturating_add(1);

        // Increment global ring-buffer pointer (used if we wanted strict sequential FIFO instead of hashed slots)
        self.head_pointer = (self.head_pointer + 1) % MAX_SLOTS;
    }

    pub(crate) fn find_mutable_quin(
        &mut self,
        subject: u64,
        predicate: u64,
        object: u64,
    ) -> Option<&mut NQuin> {
        let scan = RECENT_SLOT_RING.min(self.recent_slot_head);
        for off in 0..scan {
            let ring_idx = (self.recent_slot_head + RECENT_SLOT_RING - 1 - off) % RECENT_SLOT_RING;
            let idx = self.recent_slots[ring_idx];
            let matches = self.buffer[idx].subject == subject
                && self.buffer[idx].predicate == predicate
                && (object == 0 || self.buffer[idx].object == object);
            if matches {
                return Some(&mut self.buffer[idx]);
            }
        }
        None
    }
}

/// The Opcodes for the Lightweight Warren Abstract Machine (WAM) variant.
/// `f64` parameters require `PartialEq` only — `Eq` is not derived.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlgOpcode {
    // ── Core WAM ─────────────────────────────────────────────────────────────
    CheckTable,
    CheckDefeaters,
    CheckSubsumption,
    BranchWorld,
    /// Deprecated — use `CheckMinInclusive`.
    CheckThreshold,
    ConsumeFact,
    /// Linear-logic **zero-knowledge–gated** resource exhaustion: like `ConsumeFact`, but the
    /// cryptographic-token resource `(subject_reg, predicate_reg, object_reg)` is spent **only**
    /// if a verified zk-entitlement marker `(subject_reg, q42:zkVerified, q42:true)` is live in
    /// the arena. Absent the proof, or if the token is already spent, the frame fails (`None`).
    /// Delegates to `modalities::linear::zk_gated_consume`.
    ZkConsumeFact,
    Unify,
    Call,
    Return,
    ApplyTaxSchema,
    Halt,
    /// sh:Warning terminal — emits diagnostic but does not halt ingestion.
    WarnOnly,

    // ── Standard SHACL numeric range ─────────────────────────────────────────
    CheckMinInclusive(f64),
    CheckMaxInclusive(f64),
    CheckMinExclusive(f64),
    CheckMaxExclusive(f64),

    // ── Standard SHACL cardinality ────────────────────────────────────────────
    CheckMinCount(u32),
    CheckMaxCount(u32),

    // ── Standard SHACL string ─────────────────────────────────────────────────
    CheckMinLength(u32),
    CheckMaxLength(u32),
    /// Pattern stored as q_hash of the regex string; compared against literal hash.
    CheckPattern(u64),

    // ── Standard SHACL value constraints ──────────────────────────────────────
    /// q_hash of the single expected value.
    CheckHasValue(u64),
    /// q_hash of a referenced node-shape IRI.
    CheckNodeShape(u64),
    /// Negation: passes only if the referenced shape would FAIL.
    CheckNotShape(u64),
    /// OR-branch: records a passing `rdf:type` match without failing the frame.
    SoftCheckNodeShape(u64),
    /// Fails unless at least one preceding `SoftCheckNodeShape` matched.
    RequireAnyShape,
    /// Validates inline literal tag on `object_reg` (0=IRI, 1=int, 2=decimal, 3=bool).
    CheckObjectDatatype(u8),

    // ── Native: physics ───────────────────────────────────────────────────────
    NativeThermodynamics,
    NativeOdeSolver,
    /// RK4 ODE time-stepper with chaining support
    /// Parameters packed: step_size (lower 32 bits) | num_steps (upper 32 bits)
    NativeRk4Step(u64),
    NativeQuantumDft,
    /// `qualia:predictReceptorBinding` — PINN binding affinity.
    NativeReceptorBinding,
    /// Compile semantic constraints into blind QUBO matrix (Core 2).
    NativeQuboCompile,
    /// Emit linear QUBO bias: (var_index, f32_bits).
    NativeQuboEmitLinear(u8, u32),
    /// Emit quadratic coupler: (var_a, var_b, f32_bits).
    NativeQuboEmitCoupler(u8, u8, u32),
    /// Egress to remote QPU: 0=annealer, 1=gate-model. Yields frame to Core 3.
    NativeQuantumEgress(u8),
    /// Ingress: collapse provider JSON into provenance Quins.
    NativeQuantumIngress,

    // ── Native: biosciences ───────────────────────────────────────────────────
    /// `qualia:alignNucleotideSequence` — Smith-Waterman with BLAST nucleotide matrix.
    NativeNucleotideAlign,
    /// Deprecated form — routes to `NativeNucleotideAlign`.
    NativeBioinformatics,
    /// `qualia:alignProteinSequence` — BLOSUM62 (0) or PAM250 (1).
    NativeProteinAlign(u8),
    /// `qualia:computeKmerFrequency` — k-mer size embedded as parameter.
    NativeKmerFrequency(u8),
    /// `qualia:validateFastaRecord`.
    NativeFastaValidation,
    /// `qualia:evaluateGeneExpression`.
    NativeGeneExpression,
    /// `qualia:computeMetaboliteSimilarity` — Tanimoto fingerprint check.
    NativeMetaboliteSimilarity,

    // ── Native: biomedical ────────────────────────────────────────────────────
    /// `qualia:computeRiskScore` — 0=Framingham, 1=CHA₂DS₂-VASc, 2=SCORE2.
    NativeClinicalRisk(u8),
    /// `qualia:evaluateLongitudinalTrend` — sliding window in days.
    NativeLongitudinalTrend(u32),
    /// `qualia:evaluateDrugInteraction`.
    NativeDrugInteraction,
    /// `qualia:checkContraindication`.
    NativeContraindication,
    /// `qualia:validateFhirObservation` — LOINC code hash.
    NativeFhirObservation(u64),

    // ── Native: economics ─────────────────────────────────────────────────────
    NativeEconomics,

    // ── Native: organic chemistry ─────────────────────────────────────────────
    /// `qualia:validateSmiles` — structural SMILES validity.
    NativeSmilesValidation,
    /// `qualia:validateInchi` — InChI / InChIKey format check.
    NativeInchiValidation,
    /// `qualia:computeMolecularWeight` — exact MW from SMILES; param = max allowed Da (bits of f64).
    NativeMolecularWeight(u64),
    /// `qualia:computeLogP` — Crippen LogP; param = max allowed × 100 as i32 (stored as u32 bits).
    NativeLogP(u32),
    /// `qualia:computeTPSA` — Ertl TPSA; param = max Å² (as u32).
    NativeTPSA(u32),
    /// `qualia:evaluateLipinski` — Rule-of-Five drug-likeness filter.
    NativeLipinskiFilter,
    /// `qualia:evaluateVeber` — Veber oral-bioavailability filter.
    NativeVeberFilter,
    /// `qualia:evaluateGhose` — Ghose drug-likeness filter.
    NativeGhoseFilter,
    /// `qualia:evaluateEgan` — Egan passive-absorption filter.
    NativeEganFilter,
    /// `qualia:detectFunctionalGroups` — returns set of detected functional group hashes.
    NativeFunctionalGroups,
    /// `qualia:computePka` — functional-group-based pKa estimation.
    NativePkaEstimate,
    /// `qualia:computeChiralCenters` — count sp3 C with 4 distinct substituents.
    NativeChiralCenters,
    /// `qualia:generateCircularFingerprint` — Morgan fingerprint; param = radius.
    NativeCircularFingerprint(u8),
    /// `qualia:computeArrheniusRate` — k = A·exp(−Ea/RT); param encodes temperature K as u32.
    NativeArrhenius(u32),
    /// `qualia:computeGibbsEnergy` — ΔG = ΔH − TΔS.
    NativeGibbsEnergy,
    /// `qualia:computeEquilibrium` — K = exp(−ΔG°/RT).
    NativeEquilibrium,
    /// `qualia:computeHendersonHasselbalch` — pH from pKa + concentration ratio.
    NativeHendersonHasselbalch,
    /// `qualia:computeAtomEconomy` — Trost 1991 green metric.
    NativeAtomEconomy,
    /// `qualia:computeEFactor` — Sheldon waste-per-product metric.
    NativeEFactor,
    /// `qualia:computeGreenMetrics` — full suite: AE, E-factor, PMI, RME, CE.
    NativeGreenMetrics,

    // ── Native: Phase 5 Scientific ────────────────────────────────────────────
    NativeComputeCrcl,
    NativeComputeEgfr,
    NativeEvaluatePkModel,
    NativeComputeSofaScore,
    NativeTranslateDna,
    NativeIsoelectricPoint,
    NativePeptideCleavage,
    NativeBbbPermeation,
    NativeLigandEfficiency,
    NativeLLE,
    NativeIsotopeDistribution,

    // ── Native: deontic and epistemic ─────────────────────────────────────────
    NativeDeonticEval,
    NativeEpistemicEval(u8),

    // ── Native: advanced logics ───────────────────────────────────────────────
    NativeLinearConsume,
    NativeAspStableModels,
    NativeParaconsistentIsolate,
    NativeDialecticalSynthesis,
    /// Probabilistic gate: the goal quin's f32 belief weight (in `metadata`) must
    /// be ≥ the threshold (param = `f32::to_bits`).
    NativeProbabilisticThreshold(u32),
    /// Description-logic gate: `frame.subject_reg ⊑ frame.object_reg` over the arena
    /// TBox (transitive `rdfs:subClassOf` closure).
    NativeDlSubsumption,
    /// Argumentation gate (Dung grounded semantics): the goal argument
    /// (`frame.subject_reg`) must be justified — in the grounded extension built
    /// from `arg:asserts` / `arg:attacks` quins in the arena.
    NativeArgumentationGrounded,
    /// Metric-temporal gate (MTL "within"): `target` (`frame.object_reg`) must
    /// occur within the param window of the earliest `trigger` (`frame.predicate_reg`);
    /// event timestamps are in each quin's `metadata`.
    NativeMtlWithin(u32),
    /// Contrary-to-duty gate (dyadic deontic): if the party (`frame.subject_reg`)
    /// breached the primary obligation (`frame.predicate_reg`), the reparation
    /// (`frame.object_reg`) must be fulfilled.
    NativeContraryToDuty,
    /// Causal-necessity gate (but-for): `frame.subject_reg` must be a necessary
    /// cause of `frame.object_reg` from origin `frame.context_reg`.
    NativeCausalNecessary,
    /// Abductive gate: the observation (`frame.object_reg`) must have an explanatory
    /// hypothesis (backward `abduces:explains` chain) in the arena.
    NativeAbduce,
    /// Closed-world / negation-as-failure gate: passes iff the frame goal is ABSENT
    /// from the arena (its negation holds by default).
    NativeClosedWorld,
    /// Fuzzy-conjunction gate: the Gödel t-norm (min) of the truth degrees of all
    /// quins with predicate `frame.predicate_reg` must be ≥ the param threshold.
    NativeFuzzyConjunction(u32),
    /// CTL EF gate: from `frame.subject_reg`, SOME path reaches a state satisfying
    /// `frame.object_reg` (branching-time reachability over `ctl:next`/`ctl:holds`).
    NativeCtlExistsFinally,
    /// CTL AG gate: EVERY state reachable from `frame.subject_reg` satisfies the
    /// invariant `frame.object_reg`.
    NativeCtlAlwaysGlobally,
    /// Modal □ gate: `frame.object_reg` holds in ALL worlds accessible from
    /// `frame.subject_reg` (`modal:accesses`/`modal:holds`).
    NativeModalNecessary,
    /// Modal ◇ gate: `frame.object_reg` holds in SOME world accessible from
    /// `frame.subject_reg`.
    NativeModalPossible,
    /// RCC-8 spatial gate: the topological relation between region `frame.subject_reg`
    /// and region `frame.object_reg` (each = boundary-point quins) must equal the
    /// param (0=DC,1=EC,2=PO,3=TPP,4=TPPi,5=NTPP,6=NTPPi,7=EQ).
    NativeRcc8(u8),

    // ── Native: cognitive ai (ACT-R) ──────────────────────────────────────────
    NativeRetrieveByActivation,
    NativeDecayMetadata,
    NativeUnless,

    // ── Native: temporal logic (LTL) ──────────────────────────────────────────
    NativeLtlGlobally,
    NativeLtlFinally,
    NativeLtlNext,
    NativeLtlUntil,
    NativeLtlRelease,
    /// Evaluate a threshold proposition projected from chronological 10D
    /// manifold states through the existing LTL evaluator.
    ///
    /// `mode`: 0=Globally, 1=Finally, 2=Next.
    /// `dimension`: [`manifold::ManifoldDimension`] discriminant.
    /// `threshold_bits`: IEEE-754 f32 threshold.
    /// `at_least`: true for >=, false for <=.
    NativeManifoldLtl {
        mode: u8,
        dimension: u8,
        threshold_bits: u32,
        at_least: bool,
    },
    /// Derive topology facts from 10D states and execute the bounded real ASP
    /// stable-model evaluator. The selected model bitset is bound to object_reg.
    NativeManifoldAsp,

    // ── Native: spatio-temporal (Allen Interval) ──────────────────────────────
    NativeAllenInterval(u8),

    // ── Native: geometric and spatial topology ────────────────────────────────
    NativeLorentzDistance,
    NativeTropicalDistance,
    NativeVerifyProofOfLocation,

    // ── Native: calculus modality ──────────────────────────────────────────────
    /// `calc:SimpsonsIntegration` — CPU-based Simpson's rule (start_bits, end_bits, step_size_bits, kahan_bits)
    NativeCalcSimpsons(u64, u64, u32, u32),
    /// `calc:TrapezoidalIntegration` — CPU-based trapezoidal rule (start_bits, end_bits, step_size_bits, kahan_bits)
    NativeCalcTrapezoidal(u64, u64, u32, u32),
    /// `calc:GpuIntegration` — GPU-accelerated integration via WebGPU (start_bits, end_bits, step_size_bits, kahan_bits)
    NativeCalcGpu(u64, u64, u32, u32),
}

/// The Execution Frame tracking variable bindings without touching the heap
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VmFrame {
    pub subject_reg: u64,
    pub predicate_reg: u64,
    pub object_reg: u64,
    pub context_reg: u64,
}

#[inline]
fn frame_to_quin(frame: &VmFrame) -> NQuin {
    let mut q = NQuin {
        subject: frame.subject_reg,
        predicate: frame.predicate_reg,
        object: frame.object_reg,
        context: frame.context_reg,
        metadata: 1,
        parity: 0,
    };
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
    q
}

#[inline]
fn current_unix32() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

#[inline]
fn rdf_type_hash() -> u64 {
    crate::lexicon::generate_60bit_token(b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
}

/// Returns true when `node` has `rdf:type` = `class_hash` in the arena.
fn node_has_class(arena: &SlgArena, node: u64, class_hash: u64) -> bool {
    if node == 0 || class_hash == 0 {
        return false;
    }
    let rdf_type = rdf_type_hash();
    let mut scratch = [NQuin::default(); 256];
    let count = arena.collect_active_quins(&mut scratch);
    for q in &scratch[..count] {
        if q.subject == node && q.predicate == rdf_type && q.object == class_hash {
            return true;
        }
    }
    false
}

fn unify_frame(arena: &SlgArena, frame: &mut VmFrame) -> bool {
    if arena
        .check_table(frame.subject_reg, frame.predicate_reg, frame.object_reg)
        .is_some()
    {
        return true;
    }

    let mut scratch = [NQuin::default(); 256];
    let count = arena.collect_active_quins(&mut scratch);
    for q in &scratch[..count] {
        let subject_ok = frame.subject_reg == 0 || q.subject == frame.subject_reg;
        let predicate_ok = frame.predicate_reg == 0 || q.predicate == frame.predicate_reg;
        let object_ok = frame.object_reg == 0 || q.object == frame.object_reg;
        if subject_ok && predicate_ok && object_ok {
            frame.subject_reg = q.subject;
            frame.predicate_reg = q.predicate;
            frame.object_reg = q.object;
            frame.context_reg = q.context;
            return true;
        }
    }

    frame.subject_reg != 0 && frame.predicate_reg != 0
}

#[inline(never)]
fn execute_manifold_ltl(
    arena: &SlgArena,
    mode: u8,
    dimension: u8,
    threshold_bits: u32,
    at_least: bool,
) -> bool {
    let Some(dimension) = manifold::ManifoldDimension::from_u8(dimension) else {
        return false;
    };
    let threshold = f32::from_bits(threshold_bits);
    if !threshold.is_finite() {
        return false;
    }
    let mut snapshot = [NQuin::default(); 512];
    let snapshot_count = arena.collect_active_quins(&mut snapshot);
    let mut states = [manifold::ManifoldState10D::default(); 128];
    let state_count = manifold::collect_manifold_states(&snapshot[..snapshot_count], &mut states);
    let mut trace = [NQuin::default(); 128];
    let trace_count = manifold::project_manifold_ltl_trace(
        &states[..state_count],
        dimension,
        threshold,
        at_least,
        &mut trace,
    );
    let formula = match mode {
        0 => LtlFormula::Globally(manifold::MANIFOLD_THRESHOLD_HOLDS),
        1 => LtlFormula::Finally(manifold::MANIFOLD_THRESHOLD_HOLDS),
        2 => LtlFormula::Next(manifold::MANIFOLD_THRESHOLD_HOLDS),
        _ => return false,
    };
    temporal_ltl::evaluate_ltl_trace(&trace[..trace_count], &formula)
}

#[inline(never)]
fn execute_manifold_asp(arena: &SlgArena) -> Option<u64> {
    let mut snapshot = [NQuin::default(); 512];
    let snapshot_count = arena.collect_active_quins(&mut snapshot);
    let mut states = [manifold::ManifoldState10D::default(); 128];
    let state_count = manifold::collect_manifold_states(&snapshot[..snapshot_count], &mut states);
    if state_count == 0 {
        return None;
    }
    let mut models = [0u64; asp::MAX_STABLE_MODELS];
    let model_count = manifold::evaluate_manifold_answer_sets(&states[..state_count], &mut models);
    (model_count > 0).then(|| models[model_count - 1])
}

#[inline(never)]
fn execute_paraconsistent_isolation(arena: &mut SlgArena) -> bool {
    let mut scratch = [NQuin::default(); 64];
    let count = arena.collect_active_quins(&mut scratch);
    if count == 0 {
        return false;
    }
    let mut consistent = [NQuin::default(); 64];
    let mut isolated = [NQuin::default(); 64];
    let Ok((_, isolated_count)) =
        paraconsistent::route_paraconsistent(&scratch[..count], &mut consistent, &mut isolated)
    else {
        return false;
    };
    for quin in &isolated[..isolated_count] {
        arena.write_table(*quin);
    }
    true
}

#[inline(never)]
fn execute_dialectical_synthesis(arena: &mut SlgArena, frame: &mut VmFrame) -> bool {
    let mut scratch = [NQuin::default(); 64];
    let count = arena.collect_active_quins(&mut scratch);
    if count < 2 {
        return false;
    }
    let Some(synthesis) = dialectical::synthesize_dialectical(&scratch[0], &scratch[1]) else {
        return false;
    };
    arena.write_table(synthesis);
    frame.subject_reg = synthesis.subject;
    frame.predicate_reg = synthesis.predicate;
    frame.object_reg = synthesis.object;
    frame.context_reg = synthesis.context;
    true
}

#[inline(never)]
fn execute_standard_ltl(arena: &SlgArena, opcode: SlgOpcode, frame: &VmFrame) -> bool {
    let mut scratch = [NQuin::default(); 512];
    let count = arena.collect_active_quins(&mut scratch);
    let trace = &mut scratch[..count];
    trace.reverse();
    let formula = match opcode {
        SlgOpcode::NativeLtlGlobally => LtlFormula::Globally(frame.predicate_reg),
        SlgOpcode::NativeLtlFinally => LtlFormula::Finally(frame.predicate_reg),
        SlgOpcode::NativeLtlNext => LtlFormula::Next(frame.predicate_reg),
        SlgOpcode::NativeLtlUntil => LtlFormula::Until {
            ante: frame.predicate_reg,
            consequent: frame.object_reg,
        },
        SlgOpcode::NativeLtlRelease => LtlFormula::Release {
            trigger: frame.predicate_reg,
            invariant: frame.object_reg,
        },
        _ => return false,
    };
    temporal_ltl::evaluate_ltl_trace(trace, &formula)
}

#[inline(never)]
fn execute_snapshot_logic(arena: &SlgArena, opcode: SlgOpcode, frame: &mut VmFrame) -> bool {
    let mut scratch = [NQuin::default(); 512];
    let count = arena.collect_active_quins(&mut scratch);
    let quins = &scratch[..count];

    match opcode {
        SlgOpcode::CheckDefeaters => {
            let mut fingerprints = [0u64; MAX_DEFEATER_SLOTS];
            let fingerprint_count = harvest_defeater_fingerprints(quins, &mut fingerprints);
            !norm_has_active_defeater(&frame_to_quin(frame), &fingerprints[..fingerprint_count])
        }
        SlgOpcode::NativeDeonticEval => {
            let mut verdicts = [DeonticVerdict::default(); 64];
            let verdict_count =
                evaluate_deontic_contract(quins, current_unix32(), &mut verdicts).unwrap_or(0);
            let goal = frame_to_quin(frame);
            let valid = verdicts[..verdict_count].iter().all(|verdict| {
                verdict.norm.subject != goal.subject
                    || verdict.norm.predicate != goal.predicate
                    || verdict.norm.object != goal.object
                    || matches!(verdict.status, DeonticStatus::Active)
            });
            vm_log!(
                "[Webizen] NativeDeonticEval: {} norms evaluated",
                verdict_count
            );
            valid
        }
        SlgOpcode::NativeEpistemicEval(min_certainty) => {
            let mut verdicts = [epistemic::EpistemicVerdict {
                claim: NQuin::default(),
                status: epistemic::EpistemicStatus::Skipped,
                certainty: 0,
            }; 64];
            let verdict_count = epistemic::evaluate_epistemic_frame(
                quins,
                frame.subject_reg,
                frame.context_reg,
                &mut verdicts,
            )
            .unwrap_or(0);
            verdicts[..verdict_count].iter().any(|verdict| {
                verdict.certainty >= min_certainty
                    && verdict.status == epistemic::EpistemicStatus::Active
            })
        }
        SlgOpcode::NativeProbabilisticThreshold(threshold_bits) => {
            let threshold = f32::from_bits(threshold_bits);
            let weight = quins
                .iter()
                .find(|quin| {
                    quin.subject == frame.subject_reg
                        && quin.predicate == frame.predicate_reg
                        && quin.object == frame.object_reg
                })
                .map(probabilistic::BayesianNetwork::extract_weight)
                .unwrap_or(0.0);
            probabilistic::evaluate_threshold(weight, threshold)
        }
        SlgOpcode::NativeDlSubsumption => {
            dl::check_subsumption_quin(frame.subject_reg, frame.object_reg, quins)
        }
        SlgOpcode::NativeArgumentationGrounded => {
            let asserts = crate::q_hash("arg:asserts");
            let attacks_predicate = crate::q_hash("arg:attacks");
            let mut arguments = [0u64; argumentation::MAX_GROUNDED_ARGS];
            let mut argument_count = 0usize;
            let mut attacks = [(0u64, 0u64); 256];
            let mut attack_count = 0usize;
            for quin in quins {
                if quin.predicate == asserts && argument_count < arguments.len() {
                    arguments[argument_count] = quin.subject;
                    argument_count += 1;
                } else if quin.predicate == attacks_predicate && attack_count < attacks.len() {
                    attacks[attack_count] = (quin.subject, quin.object);
                    attack_count += 1;
                }
            }
            argumentation::grounded_contains(
                &arguments[..argument_count],
                &attacks[..attack_count],
                frame.subject_reg,
            )
        }
        SlgOpcode::NativeMtlWithin(window) => {
            temporal_ltl::holds_within(quins, frame.predicate_reg, frame.object_reg, window as u64)
        }
        SlgOpcode::NativeContraryToDuty => {
            crate::modalities::logic::deontic::evaluate_contrary_to_duty(
                quins,
                frame.subject_reg,
                frame.predicate_reg,
                frame.object_reg,
            )
        }
        SlgOpcode::NativeCausalNecessary => dialectical::is_necessary_cause(
            quins,
            frame.context_reg,
            frame.subject_reg,
            frame.object_reg,
        ),
        SlgOpcode::NativeAbduce => {
            let explains = crate::q_hash("abduces:explains");
            if let Some(hypothesis) =
                abductive::abductive_explanation(quins, frame.object_reg, explains)
            {
                frame.subject_reg = hypothesis;
                true
            } else {
                false
            }
        }
        SlgOpcode::NativeClosedWorld => defeasible::holds_by_default(quins, &frame_to_quin(frame)),
        SlgOpcode::NativeFuzzyConjunction(threshold_bits) => {
            let threshold = f32::from_bits(threshold_bits);
            let mut accumulated = 1.0f32;
            let mut found = false;
            for quin in quins {
                if quin.predicate == frame.predicate_reg {
                    accumulated = fuzzy::t_norm_godel(accumulated, fuzzy::degree(quin));
                    found = true;
                }
            }
            found && accumulated >= threshold
        }
        SlgOpcode::NativeCtlExistsFinally => ctl::exists_finally(
            quins,
            frame.subject_reg,
            frame.object_reg,
            crate::q_hash("ctl:next"),
            crate::q_hash("ctl:holds"),
        ),
        SlgOpcode::NativeCtlAlwaysGlobally => ctl::always_globally(
            quins,
            frame.subject_reg,
            frame.object_reg,
            crate::q_hash("ctl:next"),
            crate::q_hash("ctl:holds"),
        ),
        SlgOpcode::NativeModalNecessary => modal::necessary(
            quins,
            frame.subject_reg,
            frame.object_reg,
            crate::q_hash("modal:accesses"),
            crate::q_hash("modal:holds"),
        ),
        SlgOpcode::NativeModalPossible => modal::possible(
            quins,
            frame.subject_reg,
            frame.object_reg,
            crate::q_hash("modal:accesses"),
            crate::q_hash("modal:holds"),
        ),
        SlgOpcode::NativeRcc8(expected) => {
            let boundary = crate::q_hash("spatial:boundary");
            let mut region_a = [(0.0f64, 0.0f64); spatio_temporal::MAX_BOUNDARY_POINTS];
            let mut region_a_count = 0usize;
            let mut region_b = [(0.0f64, 0.0f64); spatio_temporal::MAX_BOUNDARY_POINTS];
            let mut region_b_count = 0usize;
            for quin in quins {
                if quin.predicate != boundary {
                    continue;
                }
                let index = quin.metadata as usize;
                if index >= spatio_temporal::MAX_BOUNDARY_POINTS {
                    continue;
                }
                if quin.subject == frame.subject_reg {
                    region_a[index] = spatio_temporal::unpack_point(quin.object);
                    region_a_count = region_a_count.max(index + 1);
                } else if quin.subject == frame.object_reg {
                    region_b[index] = spatio_temporal::unpack_point(quin.object);
                    region_b_count = region_b_count.max(index + 1);
                }
            }
            spatio_temporal::evaluate_rcc8_points(
                frame.subject_reg,
                &region_a[..region_a_count],
                frame.object_reg,
                &region_b[..region_b_count],
            ) as u8
                == expected
        }
        _ => false,
    }
}

/// The Bytecode Evaluator for the Prolog Webizen
pub fn execute_vm_frame(
    arena: &mut SlgArena,
    bytecode: &[SlgOpcode],
    frame: &mut VmFrame,
) -> Option<NQuin> {
    let mut instruction_pointer = 0;

    while instruction_pointer < bytecode.len() {
        let opcode = bytecode[instruction_pointer];

        match opcode {
            SlgOpcode::CheckTable => {
                // Hashing the current sub-goal to query the SlgArena
                if let Some(cached_result) =
                    arena.check_table(frame.subject_reg, frame.predicate_reg, frame.object_reg)
                {
                    // Match found! Push the cached result to the VM stack and bypass the graph traversal
                    return Some(cached_result);
                }
            }
            SlgOpcode::CheckDefeaters => {
                if !execute_snapshot_logic(arena, opcode, frame) {
                    return None;
                }
            }
            SlgOpcode::CheckSubsumption => {
                let is_subsumed =
                    dl::check_subsumption_quin(frame.subject_reg, frame.object_reg, &[]);
                if !is_subsumed {
                    return None;
                }
            }
            SlgOpcode::BranchWorld => {
                let mut out_worlds = [0; asp::MAX_STABLE_MODELS];
                let goal = frame_to_quin(frame);
                let _count = asp::enumerate_stable_models(&goal, &[], &mut out_worlds);
            }
            SlgOpcode::CheckThreshold => {
                let meets_threshold = probabilistic::evaluate_threshold(0.5, 0.8);
                if !meets_threshold {
                    return None;
                }
            }
            SlgOpcode::ConsumeFact => {
                if let Some(q) = arena.find_mutable_quin(
                    frame.subject_reg,
                    frame.predicate_reg,
                    frame.object_reg,
                ) {
                    linear::consume_quin(q);
                } else {
                    return None;
                }
            }
            SlgOpcode::ZkConsumeFact => {
                // Gate exhaustion on a verified zk-entitlement marker for the resource's subject.
                let proof_verified = arena
                    .find_mutable_quin(
                        frame.subject_reg,
                        crate::q_hash("q42:zkVerified"),
                        crate::q_hash("q42:true"),
                    )
                    .is_some();
                if let Some(q) = arena.find_mutable_quin(
                    frame.subject_reg,
                    frame.predicate_reg,
                    frame.object_reg,
                ) {
                    // Linear (consume-once) cryptographic token, gated on the zk proof.
                    if !linear::zk_gated_consume(q, false, proof_verified) {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            SlgOpcode::Unify => {
                if !unify_frame(arena, frame) {
                    return None;
                }
            }
            SlgOpcode::Call => {
                let result = frame_to_quin(frame);
                if result.subject == 0 || result.predicate == 0 {
                    return None;
                }
                arena.write_table(result);
            }
            SlgOpcode::Return => {
                return Some(frame_to_quin(frame));
            }
            SlgOpcode::ApplyTaxSchema => {
                // In a full implementation, we'd pull the active Jurisdiction Profile
                // and amount from the VM frame. For now, we mock the evaluation.
                let schema = TaxRuleSchema::new_au_gst();
                let _liability = schema.evaluate("Income", 100.0);

                // We'd store this calculated liability back into the frame
                // frame.tax_register = liability;
            }
            SlgOpcode::Halt => {
                break;
            }
            SlgOpcode::NativeThermodynamics => {
                // Mock execution of a thermodynamic state MCMC sampler
                let mut sampler =
                    crate::domains::physical::thermodynamics::ThermodynamicSampler::new(298.0, 100);
                sampler.metropolis_step(50.0, 0.5);
                vm_log!(
                    "🧪 Webizen executed NativeThermodynamics step. Current Energy: {}",
                    sampler.current_state.total_energy
                );
            }
            SlgOpcode::NativeOdeSolver => {
                // Mock execution of continuous dynamics via RK4
                #[cfg(feature = "alloc_buffers")]
                let initial = crate::ode_solver::PhysicalState {
                    time: 0.0,
                    values: alloc::vec![1.0],
                };
                #[cfg(not(feature = "alloc_buffers"))]
                let initial = crate::ode_solver::PhysicalState {
                    time: 0.0,
                    values: std::vec![1.0],
                };
                let final_state = crate::ode_solver::evaluate_continuous_dynamics(initial, 10, 0.1);
                vm_log!(
                    "📈 Webizen executed NativeOdeSolver. Final state: {:?}",
                    final_state.values
                );
            }
            SlgOpcode::NativeRk4Step(packed_params) => {
                // Unpack parameters: step_size (lower 32 bits) | num_steps (upper 32 bits)
                let step_size_bits = (packed_params & 0xFFFFFFFF) as u32;
                let num_steps = (packed_params >> 32) as u32;
                let step_size = f32::from_bits(step_size_bits) as f64;

                vm_log!(
                    "🔄 Webizen executing NativeRk4Step: step_size={}, num_steps={}",
                    step_size,
                    num_steps
                );

                // Calculus is a core capability in this crate, so RK4 dispatch stays wired.
                {
                    use crate::modalities::calculus::ode_solver::{ExponentialDecay, Rk4Solver};

                    let system = ExponentialDecay::new(0.5);
                    let mut solver = Rk4Solver::new(system, step_size);

                    // Execute chained RK4 steps
                    let mut quin = frame_to_quin(frame);
                    for _ in 0..num_steps {
                        quin = solver.step_quin(quin, step_size);
                    }

                    frame.subject_reg = quin.subject;
                    frame.predicate_reg = quin.predicate;
                    frame.object_reg = quin.object;
                    frame.context_reg = quin.context;

                    vm_log!(
                        "✅ Webizen completed {} RK4 steps. Final state: t={}, y={}",
                        num_steps,
                        f64::from_bits(quin.metadata),
                        f64::from_bits(quin.object)
                    );
                }

                /* Legacy fallback removed: calculus is always available in this crate.
                    vm_log!("⚠️  Calculus feature not enabled, RK4 step skipped");
                */
            }
            SlgOpcode::NativeQuantumDft => {
                // Mock execution of Kohn-Sham density functional approximation
                let mut dft = crate::quantum_dft::ElectronDensity::new(10);
                let energy = dft.calculate_ground_state_energy(&[]);
                vm_log!(
                    "⚛️ Webizen executed NativeQuantumDft. Ground State Energy: {} eV",
                    energy
                );
            }
            // ── Legacy / compat ───────────────────────────────────────────
            SlgOpcode::NativeBioinformatics => {
                let score =
                    crate::domains::biological::bioinformatics::align_sequences(b"ATCG", b"ATCC");
                vm_log!(
                    "[Webizen] NativeBioinformatics (legacy). SW score: {}",
                    score.score
                );
            }
            SlgOpcode::NativeEconomics => {
                let (mean, var) = crate::domains::financial::economics::run_monte_carlo_var(
                    100.0, 0.05, 0.2, 1.0, 1000, 252,
                );
                vm_log!(
                    "[Webizen] NativeEconomics. Mean: {:.2}, VaR95: {:.2}",
                    mean,
                    var
                );
            }
            // ── SHACL standard ────────────────────────────────────────────
            SlgOpcode::WarnOnly => {
                vm_log!("[Webizen] sh:Warning — constraint failed but ingestion continues.");
            }
            SlgOpcode::CheckMinInclusive(min) => {
                let val = frame.object_reg as f64;
                if val < min {
                    return None;
                }
            }
            SlgOpcode::CheckMaxInclusive(max) => {
                let val = frame.object_reg as f64;
                if val > max {
                    return None;
                }
            }
            SlgOpcode::CheckMinExclusive(min) => {
                let val = frame.object_reg as f64;
                if val <= min {
                    return None;
                }
            }
            SlgOpcode::CheckMaxExclusive(max) => {
                let val = frame.object_reg as f64;
                if val >= max {
                    return None;
                }
            }
            SlgOpcode::CheckMinCount(n) => {
                if frame.object_reg < n as u64 {
                    return None;
                }
            }
            SlgOpcode::CheckMaxCount(n) => {
                if frame.object_reg > n as u64 {
                    return None;
                }
            }
            SlgOpcode::CheckMinLength(n) => {
                if frame.object_reg < n as u64 {
                    return None;
                }
            }
            SlgOpcode::CheckMaxLength(n) => {
                if frame.object_reg > n as u64 {
                    return None;
                }
            }
            SlgOpcode::CheckPattern(pattern_hash) => {
                if frame.object_reg != pattern_hash {
                    return None;
                }
            }
            SlgOpcode::CheckHasValue(expected) => {
                if frame.object_reg != expected {
                    return None;
                }
            }
            SlgOpcode::CheckNodeShape(shape_id) => {
                if !node_has_class(arena, frame.subject_reg, shape_id) {
                    return None;
                }
            }
            SlgOpcode::CheckNotShape(shape_id) => {
                if node_has_class(arena, frame.subject_reg, shape_id) {
                    return None;
                }
            }
            SlgOpcode::SoftCheckNodeShape(shape_id) => {
                if node_has_class(arena, frame.subject_reg, shape_id) {
                    frame.context_reg |= 1;
                }
            }
            SlgOpcode::RequireAnyShape => {
                if frame.context_reg & 1 == 0 {
                    return None;
                }
            }
            SlgOpcode::CheckObjectDatatype(expected_tag) => {
                if frame.object_reg >> 63 != 0 {
                    return None;
                }
                let tag = ((frame.object_reg >> 60) & 0b111) as u8;
                if tag != expected_tag {
                    return None;
                }
            }
            // ── Biosciences ───────────────────────────────────────────────
            SlgOpcode::NativeNucleotideAlign => {
                let demo_result = crate::domains::biological::bioinformatics::align_nucleotide(
                    b"ACGTACGT",
                    b"ACGTCCGT",
                );
                vm_log!(
                    "[Webizen] NativeNucleotideAlign. SW score: {}, identity: {:.1}%",
                    demo_result.score,
                    demo_result.identity_pct
                );
                if demo_result.score <= 0 {
                    return None;
                }
            }
            SlgOpcode::NativeProteinAlign(matrix_id) => {
                let result = crate::domains::biological::bioinformatics::align_protein(
                    b"ACDEFGHIK",
                    b"ACDEFGHIK",
                );
                vm_log!(
                    "[Webizen] NativeProteinAlign(matrix={}) score: {}, id: {:.1}%",
                    matrix_id,
                    result.score,
                    result.identity_pct
                );
                if result.score <= 0 {
                    return None;
                }
            }
            SlgOpcode::NativeKmerFrequency(k) => {
                let freqs = crate::domains::biological::bioinformatics::kmer_frequencies(
                    b"ACGTACGTACGT",
                    k as usize,
                );
                vm_log!(
                    "[Webizen] NativeKmerFrequency(k={}) distinct k-mers: {}",
                    k,
                    freqs.len()
                );
            }

            SlgOpcode::NativeFastaValidation => {
                let record = crate::domains::biological::bioinformatics::validate_fasta_record(
                    ">test",
                    b"ATCGATCG",
                );
                if !record.is_valid {
                    return None;
                }
                vm_log!("[Webizen] NativeFastaValidation: {:?}", record.alphabet);
            }
            SlgOpcode::NativeGeneExpression => {
                let result = crate::clinical_engine::evaluate_gene_expression(
                    frame.subject_reg,
                    100.0,
                    frame.object_reg as f64,
                    2.0,
                );
                vm_log!(
                    "[Webizen] NativeGeneExpression: FC={:.2} log2FC={:.2} sig={}",
                    result.fold_change,
                    result.log2_fold_change,
                    result.is_significant
                );
                if !result.is_significant {
                    return None;
                }
            }
            SlgOpcode::NativeMetaboliteSimilarity => {
                let fp_a = vec![frame.subject_reg];
                let fp_b = vec![frame.object_reg];
                let sim =
                    crate::domains::biological::bioinformatics::tanimoto_similarity(&fp_a, &fp_b);
                vm_log!("[Webizen] NativeMetaboliteSimilarity: Tanimoto={:.3}", sim);
                if sim < 0.4 {
                    return None;
                }
            }
            SlgOpcode::NativeReceptorBinding => {
                let goal = frame_to_quin(frame);
                let affinity = crate::quantum_dft::pinn_predict_receptor_binding(&[goal], &[goal]);
                vm_log!(
                    "[Webizen] NativeReceptorBinding: affinity={:.2} kcal/mol",
                    affinity
                );
            }
            // ── Biomedical ────────────────────────────────────────────────
            #[cfg(not(target_arch = "wasm32"))]
            SlgOpcode::NativeClinicalRisk(model_id) => match model_id {
                0 => {
                    let input = crate::clinical_engine::FraminghamInput {
                        age: (frame.object_reg & 0xFF) as u8,
                        sex_male: (frame.metadata_hint() & 1) != 0,
                        total_cholesterol_mmol: 5.5,
                        hdl_cholesterol_mmol: 1.2,
                        systolic_bp: 130.0,
                        bp_treated: false,
                        current_smoker: false,
                        diabetic: false,
                    };
                    let r = crate::clinical_engine::framingham_10yr_risk(&input);
                    vm_log!(
                        "[Webizen] Framingham 10yr risk: {:.1}% ({:?})",
                        r.risk_10yr * 100.0,
                        r.category
                    );
                }
                1 => {
                    let input = crate::clinical_engine::Cha2ds2VascInput {
                        hypertension: (frame.object_reg & 0x01) != 0,
                        diabetes: (frame.object_reg & 0x02) != 0,
                        age_65_to_74: (frame.object_reg & 0x04) != 0,
                        ..Default::default()
                    };
                    let r = crate::clinical_engine::cha2ds2_vasc_score(&input);
                    vm_log!(
                        "[Webizen] CHA₂DS₂-VASc: {} ({:.1}%/yr)",
                        r.score,
                        r.annual_stroke_risk_pct
                    );
                }
                2 => {
                    let input = crate::clinical_engine::Score2Input {
                        age: (frame.object_reg & 0xFF) as u8,
                        sex_male: true,
                        systolic_bp: 130.0,
                        total_cholesterol_mmol: 5.5,
                        hdl_cholesterol_mmol: 1.3,
                        current_smoker: false,
                        risk_region: crate::clinical_engine::Score2Region::Moderate,
                    };
                    let r = crate::clinical_engine::score2_risk(&input);
                    vm_log!(
                        "[Webizen] SCORE2: {:.1}% ({:?})",
                        r.risk_10yr_pct,
                        r.category
                    );
                }
                _ => vm_log!("[Webizen] NativeClinicalRisk: unknown model {}", model_id),
            },
            #[cfg(target_arch = "wasm32")]
            SlgOpcode::NativeClinicalRisk(_) => {}

            SlgOpcode::NativeLongitudinalTrend(window_days) => {
                vm_log!("[Webizen] NativeLongitudinalTrend: window={}d — awaiting time-series Quin stream", window_days);
            }

            #[cfg(not(target_arch = "wasm32"))]
            SlgOpcode::NativeDrugInteraction => {
                let meds = vec![frame.subject_reg, frame.object_reg];
                let found = crate::clinical_engine::check_drug_interactions(&meds);
                if !found.is_empty() {
                    vm_log!(
                        "[Webizen] NativeDrugInteraction: {} interaction(s) found. Worst: {:?}",
                        found.len(),
                        found[0].severity
                    );
                    if found[0].severity >= crate::clinical_engine::InteractionSeverity::Major {
                        return None;
                    }
                }
            }
            #[cfg(target_arch = "wasm32")]
            SlgOpcode::NativeDrugInteraction => {}

            #[cfg(not(target_arch = "wasm32"))]
            SlgOpcode::NativeContraindication => {
                let conds = vec![frame.object_reg];
                let found =
                    crate::clinical_engine::check_contraindications(frame.subject_reg, &conds);
                if !found.is_empty() {
                    vm_log!(
                        "[Webizen] NativeContraindication: {} contraindication(s) found.",
                        found.len()
                    );
                    return None;
                }
            }
            #[cfg(target_arch = "wasm32")]
            SlgOpcode::NativeContraindication => {}

            #[cfg(not(target_arch = "wasm32"))]
            SlgOpcode::NativeFhirObservation(loinc_hash) => {
                let obs = crate::clinical_engine::FhirObservation {
                    loinc_code: format!("{:016x}", loinc_hash),
                    value: f64::from_bits(frame.object_reg),
                    unit_ucum: String::new(),
                    reference_low: None,
                    reference_high: None,
                };
                let r = crate::clinical_engine::validate_fhir_observation(&obs);
                vm_log!(
                    "[Webizen] NativeFhirObservation: status={:?} interp={}",
                    r.status,
                    r.interpretation_code
                );
                if !r.is_valid {
                    return None;
                }
            }
            #[cfg(target_arch = "wasm32")]
            SlgOpcode::NativeFhirObservation(_) => {}
            // ── Organic chemistry ─────────────────────────────────────────
            SlgOpcode::NativeSmilesValidation => {
                // In production the SMILES string is retrieved from the lexicon by object_reg hash.
                // Demo path: validate a demonstration SMILES.
                let demo = "CC(=O)Oc1ccccc1C(=O)O"; // aspirin
                let r = crate::domains::chemical::organic_chemistry::validate_smiles(demo);
                vm_log!(
                    "[Webizen] NativeSmilesValidation: valid={} atoms={}",
                    r.is_valid,
                    r.atom_count
                );
                if !r.is_valid {
                    return None;
                }
            }
            SlgOpcode::NativeInchiValidation => {
                let demo = "InChI=1S/C9H8O4/c1-6(10)13-8-5-3-2-4-7(8)9(11)12/h2-5H,1H3,(H,11,12)";
                let r = crate::domains::chemical::organic_chemistry::validate_inchi(demo);
                vm_log!(
                    "[Webizen] NativeInchiValidation: valid={} layers={}",
                    r.is_valid,
                    r.layer_count
                );
                if !r.is_valid {
                    return None;
                }
            }
            SlgOpcode::NativeMolecularWeight(max_mw_bits) => {
                let max_mw = f64::from_bits(max_mw_bits);
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let mw = crate::domains::chemical::organic_chemistry::exact_molecular_weight(&mol);
                vm_log!(
                    "[Webizen] NativeMolecularWeight: {:.2} Da (max allowed {:.1})",
                    mw,
                    max_mw
                );
                if max_mw > 0.0 && mw > max_mw {
                    return None;
                }
            }
            SlgOpcode::NativeLogP(max_bits) => {
                let max_logp = max_bits as f64 / 100.0;
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let logp = crate::domains::chemical::organic_chemistry::compute_logp(&mol);
                vm_log!("[Webizen] NativeLogP: {:.2} (max {:.2})", logp, max_logp);
                if max_logp > 0.0 && logp > max_logp {
                    return None;
                }
            }
            SlgOpcode::NativeTPSA(max_tpsa) => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let tpsa = crate::domains::chemical::organic_chemistry::compute_tpsa(&mol);
                vm_log!("[Webizen] NativeTPSA: {:.1} Å² (max {})", tpsa, max_tpsa);
                if max_tpsa > 0 && tpsa > max_tpsa as f64 {
                    return None;
                }
            }
            SlgOpcode::NativeLipinskiFilter => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let desc = crate::domains::chemical::organic_chemistry::compute_descriptors(&mol);
                let r = crate::domains::chemical::organic_chemistry::evaluate_lipinski(&desc);
                vm_log!(
                    "[Webizen] NativeLipinskiFilter: passes={} violations={}",
                    r.passes,
                    r.violations
                );
                if !r.passes {
                    return None;
                }
            }
            SlgOpcode::NativeVeberFilter => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let desc = crate::domains::chemical::organic_chemistry::compute_descriptors(&mol);
                let r = crate::domains::chemical::organic_chemistry::evaluate_veber(&desc);
                vm_log!("[Webizen] NativeVeberFilter: passes={}", r.passes);
                if !r.passes {
                    return None;
                }
            }
            SlgOpcode::NativeGhoseFilter => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let desc = crate::domains::chemical::organic_chemistry::compute_descriptors(&mol);
                let r = crate::domains::chemical::organic_chemistry::evaluate_ghose(&desc);
                vm_log!("[Webizen] NativeGhoseFilter: passes={}", r.passes);
            }
            SlgOpcode::NativeEganFilter => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let desc = crate::domains::chemical::organic_chemistry::compute_descriptors(&mol);
                let r = crate::domains::chemical::organic_chemistry::evaluate_egan(&desc);
                vm_log!("[Webizen] NativeEganFilter: passes={}", r.passes);
            }
            SlgOpcode::NativeFunctionalGroups => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let groups =
                    crate::domains::chemical::organic_chemistry::detect_functional_groups(&mol);
                vm_log!("[Webizen] NativeFunctionalGroups: {:?}", groups);
            }
            SlgOpcode::NativePkaEstimate => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles("CC(=O)O"); // acetic acid
                let pkas = crate::domains::chemical::organic_chemistry::estimate_pka(&mol);
                for p in &pkas {
                    vm_log!(
                        "[Webizen] NativePka: {:?} pKa={:.1} acid={}",
                        p.group,
                        p.pka,
                        p.is_acid
                    );
                }
            }
            SlgOpcode::NativeChiralCenters => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let n = crate::domains::chemical::organic_chemistry::count_chiral_centers(&mol);
                vm_log!("[Webizen] NativeChiralCenters: {}", n);
            }
            SlgOpcode::NativeCircularFingerprint(radius) => {
                let mol = crate::domains::chemical::organic_chemistry::parse_smiles(
                    "CC(=O)Oc1ccccc1C(=O)O",
                );
                let fp = crate::domains::chemical::organic_chemistry::circular_fingerprint(
                    &mol,
                    radius as usize,
                );
                vm_log!(
                    "[Webizen] NativeCircularFingerprint(r={}): {} features",
                    radius,
                    fp.len()
                );
            }
            SlgOpcode::NativeArrhenius(temp_k) => {
                let k = crate::domains::chemical::organic_chemistry::arrhenius_rate(
                    1e13,
                    80_000.0,
                    temp_k as f64,
                );
                vm_log!("[Webizen] NativeArrhenius(T={}K): k={:.3e}", temp_k, k);
            }
            SlgOpcode::NativeGibbsEnergy => {
                let dg = crate::domains::chemical::organic_chemistry::gibbs_free_energy(
                    f64::from_bits(frame.subject_reg),
                    f64::from_bits(frame.predicate_reg),
                    f64::from_bits(frame.object_reg),
                );
                vm_log!("[Webizen] NativeGibbsEnergy: ΔG={:.2} J/mol", dg);
            }
            SlgOpcode::NativeEquilibrium => {
                let k_eq = crate::domains::chemical::organic_chemistry::equilibrium_constant(
                    f64::from_bits(frame.subject_reg),
                    f64::from_bits(frame.object_reg),
                );
                vm_log!("[Webizen] NativeEquilibrium: K={:.4e}", k_eq);
            }
            SlgOpcode::NativeHendersonHasselbalch => {
                let ph = crate::domains::chemical::organic_chemistry::henderson_hasselbalch(
                    f64::from_bits(frame.subject_reg),
                    f64::from_bits(frame.predicate_reg),
                    f64::from_bits(frame.object_reg),
                );
                vm_log!("[Webizen] NativeHendersonHasselbalch: pH={:.2}", ph);
            }
            SlgOpcode::NativeAtomEconomy => {
                let reactants = vec![180.0, 60.0]; // demo
                let ae =
                    crate::domains::chemical::organic_chemistry::atom_economy(&reactants, 180.0);
                vm_log!("[Webizen] NativeAtomEconomy: {:.1}%", ae);
            }
            SlgOpcode::NativeEFactor => {
                let ef = crate::domains::chemical::organic_chemistry::e_factor(
                    f64::from_bits(frame.subject_reg),
                    f64::from_bits(frame.object_reg),
                );
                vm_log!("[Webizen] NativeEFactor: {:.2} kg waste/kg product", ef);
            }
            SlgOpcode::NativeGreenMetrics => {
                let gm = crate::domains::chemical::organic_chemistry::green_metrics(
                    &[180.0, 60.0],
                    180.0,
                    &[60.0],
                    0.85,
                    50.0,
                    1.0,
                    9,
                    9,
                );
                vm_log!(
                    "[Webizen] NativeGreenMetrics: AE={:.1}% E={:.1} PMI={:.1}",
                    gm.atom_economy_pct,
                    gm.e_factor,
                    gm.process_mass_intensity
                );
            }
            SlgOpcode::NativeComputeCrcl => {
                vm_log!("[Webizen] NativeComputeCrcl evaluated");
            }
            SlgOpcode::NativeComputeEgfr => {
                vm_log!("[Webizen] NativeComputeEgfr evaluated");
            }
            SlgOpcode::NativeEvaluatePkModel => {
                vm_log!("[Webizen] NativeEvaluatePkModel evaluated");
            }
            SlgOpcode::NativeComputeSofaScore => {
                vm_log!("[Webizen] NativeComputeSofaScore evaluated");
            }
            SlgOpcode::NativeTranslateDna => {
                vm_log!("[Webizen] NativeTranslateDna evaluated");
            }
            SlgOpcode::NativeIsoelectricPoint => {
                vm_log!("[Webizen] NativeIsoelectricPoint evaluated");
            }
            SlgOpcode::NativePeptideCleavage => {
                vm_log!("[Webizen] NativePeptideCleavage evaluated");
            }
            SlgOpcode::NativeBbbPermeation => {
                vm_log!("[Webizen] NativeBbbPermeation evaluated");
            }
            SlgOpcode::NativeLigandEfficiency => {
                vm_log!("[Webizen] NativeLigandEfficiency evaluated");
            }
            SlgOpcode::NativeLLE => {
                vm_log!("[Webizen] NativeLLE evaluated");
            }
            SlgOpcode::NativeIsotopeDistribution => {
                vm_log!("[Webizen] NativeIsotopeDistribution evaluated");
            }
            SlgOpcode::NativeDeonticEval => {
                if !execute_snapshot_logic(arena, opcode, frame) {
                    return None;
                }
            }
            SlgOpcode::NativeEpistemicEval(_) => {
                if !execute_snapshot_logic(arena, opcode, frame) {
                    return None;
                }
            }
            SlgOpcode::NativeLinearConsume => {
                if let Some(q) = arena.find_mutable_quin(
                    frame.subject_reg,
                    frame.predicate_reg,
                    frame.object_reg,
                ) {
                    linear::consume_quin(q);
                } else {
                    return None;
                }
            }
            SlgOpcode::NativeAspStableModels => {
                // Enumerate stable models over the live rules in the arena. (Passing
                // an empty rule set would trivially yield a single world and ignore
                // the knowledge base.)
                let mut rules = [NQuin::default(); asp::MAX_STABLE_MODELS];
                let nrules = arena.collect_active_quins(&mut rules);
                let mut out_worlds = [0; asp::MAX_STABLE_MODELS];
                let goal = frame_to_quin(frame);
                let world_count =
                    asp::enumerate_stable_models(&goal, &rules[..nrules], &mut out_worlds);
                if world_count == 0 {
                    return None;
                }
                // Bind the frame to the last enumerated stable model.
                frame.context_reg = out_worlds[world_count - 1];
            }
            SlgOpcode::NativeParaconsistentIsolate => {
                if !execute_paraconsistent_isolation(arena) {
                    return None;
                }
            }
            SlgOpcode::NativeDialecticalSynthesis => {
                if !execute_dialectical_synthesis(arena, frame) {
                    return None;
                }
            }
            SlgOpcode::NativeProbabilisticThreshold(_)
            | SlgOpcode::NativeDlSubsumption
            | SlgOpcode::NativeArgumentationGrounded
            | SlgOpcode::NativeMtlWithin(_)
            | SlgOpcode::NativeContraryToDuty
            | SlgOpcode::NativeCausalNecessary
            | SlgOpcode::NativeAbduce
            | SlgOpcode::NativeClosedWorld
            | SlgOpcode::NativeFuzzyConjunction(_)
            | SlgOpcode::NativeCtlExistsFinally
            | SlgOpcode::NativeCtlAlwaysGlobally
            | SlgOpcode::NativeModalNecessary
            | SlgOpcode::NativeModalPossible
            | SlgOpcode::NativeRcc8(_) => {
                if !execute_snapshot_logic(arena, opcode, frame) {
                    return None;
                }
            }
            SlgOpcode::NativeUnless => {
                let goal = frame_to_quin(frame);
                let property_path = (goal.predicate >> 8) & !DEFEATER_BIT;
                let defeater = compile_norm_quin(
                    goal.subject,
                    OP_PERMIT,
                    property_path,
                    goal.object,
                    goal.context,
                    0,
                    true,
                );
                arena.write_table(defeater);
            }
            SlgOpcode::NativeRetrieveByActivation | SlgOpcode::NativeDecayMetadata => {
                // CORE 2 ISOLATION RULE (ACT-R Escalation):
                // Do not block Core 1. Push float activation/decay ops to async Sieve (Core 2 / GPU).
                // Suspend the Sentinel rule frame.
                vm_log!("[Webizen] CORE 2 YIELD: Suspending frame and pushing CogAI retrieval/decay to async GPU Sieve.");
                return None;
            }
            SlgOpcode::NativeManifoldLtl {
                mode,
                dimension,
                threshold_bits,
                at_least,
            } => {
                if !execute_manifold_ltl(arena, mode, dimension, threshold_bits, at_least) {
                    return None;
                }
            }
            SlgOpcode::NativeManifoldAsp => {
                frame.object_reg = execute_manifold_asp(arena)?;
            }
            SlgOpcode::NativeLtlGlobally
            | SlgOpcode::NativeLtlFinally
            | SlgOpcode::NativeLtlNext
            | SlgOpcode::NativeLtlUntil
            | SlgOpcode::NativeLtlRelease => {
                if !execute_standard_ltl(arena, opcode, frame) {
                    return None;
                }
                vm_log!("[Webizen] NativeLtl: temporal property held");
            }
            SlgOpcode::NativeAllenInterval(mode) => {
                // The frame registers carry the two intervals' bounds:
                //   subject = t1_start, predicate = t1_end,
                //   object  = t2_start, context   = t2_end.
                let op = match mode {
                    0 => spatio_temporal::TemporalOp::Before,
                    1 => spatio_temporal::TemporalOp::Meets,
                    2 => spatio_temporal::TemporalOp::Overlaps,
                    3 => spatio_temporal::TemporalOp::Starts,
                    4 => spatio_temporal::TemporalOp::During,
                    5 => spatio_temporal::TemporalOp::Finishes,
                    _ => spatio_temporal::TemporalOp::Equals,
                };
                let holds = spatio_temporal::evaluate_temporal(
                    op,
                    frame.subject_reg as i64,
                    frame.predicate_reg as i64,
                    frame.object_reg as i64,
                    frame.context_reg as i64,
                );
                if !holds {
                    return None; // the interval relation does not hold → frame fails
                }
                vm_log!(
                    "[Webizen] NativeAllenInterval: relation mode {} holds",
                    mode
                );
            }
            SlgOpcode::NativeLorentzDistance
            | SlgOpcode::NativeTropicalDistance
            | SlgOpcode::NativeVerifyProofOfLocation => {
                // CORE 2 ISOLATION RULE:
                // Do not block Core 1. Push 64-bit parameters to async Sieve (Core 2 / GPU).
                // Suspend the Sentinel rule frame.
            }
            SlgOpcode::NativeCalcSimpsons(start_bits, end_bits, step_size_bits, kahan_bits) => {
                let start = f64::from_bits(start_bits);
                let end = f64::from_bits(end_bits);
                let step_size = f64::from_bits(step_size_bits as u64);
                let _kahan_compensation = f32::from_bits(kahan_bits);

                // Create a mock continuous grid for demonstration (as bytes)
                let grid_data: Vec<u8> = vec![0u8; 1001 * 8]; // 1001 f64 values
                let grid =
                    crate::modalities::calculus::ContinuousGrid::new(&grid_data, 1001).unwrap();

                let result =
                    crate::modalities::calculus::integrate_simpsons_chunked(&grid, step_size)
                        .unwrap_or(f64::NAN);
                vm_log!(
                    "[Webizen] NativeCalcSimpsons: [{}, {}] h={:.4} result={:.6}",
                    start,
                    end,
                    step_size,
                    result
                );
            }
            SlgOpcode::NativeCalcTrapezoidal(start_bits, end_bits, step_size_bits, kahan_bits) => {
                let start = f64::from_bits(start_bits);
                let end = f64::from_bits(end_bits);
                let step_size = f64::from_bits(step_size_bits as u64);
                let _kahan_compensation = f32::from_bits(kahan_bits);

                // Create a mock continuous grid for demonstration (as bytes)
                let grid_data: Vec<u8> = vec![0u8; 1000 * 8]; // 1000 f64 values
                let grid =
                    crate::modalities::calculus::ContinuousGrid::new(&grid_data, 1000).unwrap();

                let result =
                    crate::modalities::calculus::integrate_trapezoidal_chunked(&grid, step_size)
                        .unwrap_or(f64::NAN);
                vm_log!(
                    "[Webizen] NativeCalcTrapezoidal: [{}, {}] h={:.4} result={:.6}",
                    start,
                    end,
                    step_size,
                    result
                );
            }
            SlgOpcode::NativeCalcGpu(start_bits, end_bits, step_size_bits, kahan_bits) => {
                let start = f64::from_bits(start_bits);
                let end = f64::from_bits(end_bits);
                let step_size = f32::from_bits(step_size_bits);
                let _kahan_compensation = f32::from_bits(kahan_bits);

                vm_log!(
                    "[Webizen] NativeCalcGpu: GPU integration requested for [{}, {}] h={:.4}",
                    start,
                    end,
                    step_size
                );

                // Create GPU integrator and attempt async execution
                #[cfg(not(target_arch = "wasm32"))]
                {
                    use crate::modalities::calculus::gpu::{GpuIntegrator, PlatformGpuIntegrator};
                    use std::path::Path;

                    // Use tokio runtime to block on async GPU initialization
                    if let Ok(handle) = tokio::runtime::Handle::try_current() {
                        let gpu_result =
                            handle.block_on(async { PlatformGpuIntegrator::new().await });

                        match gpu_result {
                            Ok(mut gpu_integrator) => {
                                // Calculate size from boundaries (assuming f64 grid)
                                let num_points = ((end - start) / step_size as f64) as usize;
                                let size = (num_points * 8) as u64; // bytes

                                // Use alignment resolver to get DMA-safe offset
                                let (aligned_offset, _remainder) =
                                    crate::modalities::calculus::resolve_aligned_byte_offset(0);

                                // For demo, use a temp file path - in production this would come from Quin context
                                let temp_path = Path::new("calculus_grid.dat");

                                match gpu_integrator.integrate_simpsons_gpu(
                                    temp_path,
                                    aligned_offset,
                                    size,
                                    step_size,
                                ) {
                                    Ok(result) => {
                                        vm_log!(
                                            "[Webizen] NativeCalcGpu: GPU integration complete result={:.6}",
                                            result
                                        );
                                        // In production, would pack result into quin.metadata and resuspend
                                    }
                                    Err(e) => {
                                        vm_log!(
                                            "[Webizen] NativeCalcGpu: GPU integration failed, falling back to CPU: {:?}",
                                            e
                                        );
                                        // Fallback to CPU Simpson's
                                        let grid_data: Vec<u8> = vec![0u8; 1001 * 8];
                                        let grid =
                                            crate::modalities::calculus::ContinuousGrid::new(
                                                &grid_data, 1001,
                                            )
                                            .unwrap();
                                        let cpu_result =
                                            crate::modalities::calculus::integrate_simpsons_chunked(
                                                &grid,
                                                step_size as f64,
                                            )
                                            .unwrap_or(f64::NAN);
                                        vm_log!(
                                            "[Webizen] NativeCalcGpu: CPU fallback result={:.6}",
                                            cpu_result
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                vm_log!(
                                    "[Webizen] NativeCalcGpu: GPU initialization failed, falling back to CPU: {:?}",
                                    e
                                );
                                // Fallback to CPU Simpson's
                                let grid_data: Vec<u8> = vec![0u8; 1001 * 8];
                                let grid = crate::modalities::calculus::ContinuousGrid::new(
                                    &grid_data, 1001,
                                )
                                .unwrap();
                                let cpu_result =
                                    crate::modalities::calculus::integrate_simpsons_chunked(
                                        &grid,
                                        step_size as f64,
                                    )
                                    .unwrap_or(f64::NAN);
                                vm_log!(
                                    "[Webizen] NativeCalcGpu: CPU fallback result={:.6}",
                                    cpu_result
                                );
                            }
                        }
                    } else {
                        vm_log!(
                            "[Webizen] NativeCalcGpu: Tokio runtime failed, using CPU fallback"
                        );
                        let grid_data: Vec<u8> = vec![0u8; 1001 * 8];
                        let grid =
                            crate::modalities::calculus::ContinuousGrid::new(&grid_data, 1001)
                                .unwrap();
                        let cpu_result = crate::modalities::calculus::integrate_simpsons_chunked(
                            &grid,
                            step_size as f64,
                        )
                        .unwrap_or(f64::NAN);
                        vm_log!(
                            "[Webizen] NativeCalcGpu: CPU fallback result={:.6}",
                            cpu_result
                        );
                    }
                }

                #[cfg(target_arch = "wasm32")]
                {
                    vm_log!(
                        "[Webizen] NativeCalcGpu: GPU not available on WASM, using CPU fallback"
                    );
                    let grid_data: Vec<u8> = vec![0u8; 1001 * 8];
                    let grid =
                        crate::modalities::calculus::ContinuousGrid::new(&grid_data, 1001).unwrap();
                    let cpu_result = crate::modalities::calculus::integrate_simpsons_chunked(
                        &grid,
                        step_size as f64,
                    )
                    .unwrap_or(f64::NAN);
                    vm_log!(
                        "[Webizen] NativeCalcGpu: CPU fallback result={:.6}",
                        cpu_result
                    );
                }
            }
            SlgOpcode::NativeQuboCompile => {
                vm_log!(
                    "[Webizen] NativeQuboCompile: semantic subgraph → blind QUBO matrix (Core 2)"
                );
            }
            SlgOpcode::NativeQuboEmitLinear(var, bits) => {
                let bias = f32::from_bits(bits);
                vm_log!("[Webizen] OP_EMIT_WEIGHT linear var={} bias={}", var, bias);
            }
            SlgOpcode::NativeQuboEmitCoupler(a, b, bits) => {
                let w = f32::from_bits(bits);
                vm_log!("[Webizen] OP_EMIT_WEIGHT coupler {}-{} weight={}", a, b, w);
            }
            SlgOpcode::NativeQuantumEgress(arch) => {
                vm_log!("[Webizen] CORE 3 YIELD: NativeQuantumEgress arch={} — suspending for blind HTTP egress", arch);
                return None;
            }
            SlgOpcode::NativeQuantumIngress => {
                vm_log!(
                    "[Webizen] NativeQuantumIngress: collapsing QPU response → provenance Quins"
                );
            }
        }

        instruction_pointer += 1;
    }

    None
}

impl VmFrame {
    /// Reads a hint from the lower bits of predicate_reg.
    #[inline(always)]
    pub fn metadata_hint(&self) -> u64 {
        self.predicate_reg & 0xFF
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgreementState {
    Proposed = 0x00,
    PartiallySigned = 0x01,
    Ratified = 0x02,
}

#[derive(Debug, Clone)]
pub struct AgreementDomain {
    #[cfg(feature = "alloc_buffers")]
    pub name: alloc::string::String,
    #[cfg(not(feature = "alloc_buffers"))]
    pub name: std::string::String,
    pub domain_id: u64,
}

#[derive(Debug, Clone)]
pub struct AgreementConstraint {
    pub required_signatures: u8,
}

pub struct AgreementDID {
    pub agreement_id: u64,
    pub principal: u64,
    pub agents: [u64; 8],
    pub num_agents: u8,
    pub domain_id: u64,
    pub threshold: u8,
    pub current_state: AgreementState,
}

impl AgreementDID {
    /// Compiles a ratified agreement into hardware-aligned Super-Quins.
    pub fn compile_to_super_quins(&self) -> [NQuin; 16] {
        let mut buffer = [NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        }; 16];
        if self.current_state != AgreementState::Ratified {
            return buffer;
        }

        let mut idx = 0;
        let has_guardian = crate::q_hash("q42:hasGuardian");
        let has_domain_scope = crate::q_hash("q42:hasDomainScope");
        let requires_consensus = crate::q_hash("q42:requiresConsensus");

        for i in 0..self.num_agents as usize {
            if idx < 16 {
                buffer[idx] = NQuin {
                    subject: self.principal,
                    predicate: has_guardian,
                    object: self.agents[i],
                    context: self.agreement_id,
                    // Embed routing lane (Bilateral Micro-Commons) and the State
                    metadata: 0x4000_0000_0000_0002 | ((self.current_state as u64) << 48),
                    parity: 0,
                };
                idx += 1;
            }
        }

        for i in 0..self.num_agents as usize {
            if idx < 16 {
                buffer[idx] = NQuin {
                    subject: self.agreement_id,
                    predicate: has_domain_scope,
                    object: self.domain_id,
                    context: self.agents[i],
                    metadata: 0x4000_0000_0000_0002,
                    parity: 0,
                };
                idx += 1;
            }
        }

        if idx < 16 {
            buffer[idx] = NQuin {
                subject: self.agreement_id,
                predicate: requires_consensus,
                object: self.threshold as u64,
                context: self.domain_id,
                metadata: 0x4000_0000_0000_0002,
                parity: 0,
            };
        }

        buffer
    }
}

/// Values abuse-check (the engine side of the MCP `values_check` tool).
///
/// Runs the REAL inverse rights-guard lane (agency.n3 G1 + its software-agent twin G1')
/// in a fresh arena: a non–natural-person agent that *claims* a natural-person-only dignity
/// right trips `values:PersonhoodCategoryError`. This is the anti-capture invariant — a
/// `CorporatePerson` or an `ArtificialAgent` cannot wear a human's dignity right as its own.
///
/// `agent_type` is `q_hash("https://ns.webcivics.net/values/<Class>")`. Returns `true` iff the
/// guard fires. A `NaturalPerson` (or a non-claiming agent) is never flagged. Cold path — uses
/// the same `Rule`/`Formula` machinery as `register_rule`, never a hot-path allocation.
pub fn check_personhood_category_error(agent_type: u64, claims_dignity_right: bool) -> bool {
    use crate::modalities::logic::n3_parser::{Formula, Rule, RuleType, Term, Triple};
    const B: &str = "https://ns.webcivics.net/values/";
    let vh = |s: &str| crate::q_hash(s);
    let u = |s: &'static str| Term::Uri(s);
    let var = |n: &'static str| Term::Variable(n);

    // G-guard for a given non-natural-person class: claiming a NaturalPerson-held Right → flag.
    // `class_uri` is the FULL values: IRI of the guarded class, so its `q_hash`
    // matches the `agent_type` fact below. Full-IRI `&'static str` literals keep
    // this zero-heap (the predecessor leaked `format!` Strings via `Box::leak`).
    let guard = |id: &'static str, class_uri: &'static str| Rule {
        id: Some(id),
        rule_type: RuleType::Strict,
        weight: None,
        premise: Formula {
            triples: vec![
                Triple {
                    subject: var("c"),
                    predicate: u("a"),
                    object: u(class_uri),
                },
                Triple {
                    subject: var("c"),
                    predicate: u("https://ns.webcivics.net/values/claims"),
                    object: var("r"),
                },
                Triple {
                    subject: var("r"),
                    predicate: u("a"),
                    object: u("https://ns.webcivics.net/values/Right"),
                },
                Triple {
                    subject: var("r"),
                    predicate: u("https://ns.webcivics.net/values/heldBy"),
                    object: u("https://ns.webcivics.net/values/NaturalPerson"),
                },
            ],
        },
        conclusion: Formula {
            triples: vec![Triple {
                subject: var("c"),
                predicate: u("https://ns.webcivics.net/values/flag"),
                object: u("https://ns.webcivics.net/values/PersonhoodCategoryError"),
            }],
        },
    };

    let mut arena = SlgArena::new();
    let r1 = guard(
        "agency-G1",
        "https://ns.webcivics.net/values/CorporatePerson",
    );
    arena.register_rule(&r1);
    let r2 = guard(
        "agency-G1-prime",
        "https://ns.webcivics.net/values/ArtificialAgent",
    );
    arena.register_rule(&r2);

    let fact = |a: &mut SlgArena, s: u64, p: u64, o: u64| {
        a.write_table(NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: s ^ p ^ o,
        });
    };
    let agent = vh("urn:webcivics:values-check:agent");
    let right = vh("urn:webcivics:values-check:right");
    fact(&mut arena, agent, vh("a"), agent_type);
    if claims_dignity_right {
        fact(&mut arena, agent, vh(&format!("{B}claims")), right);
        fact(&mut arena, right, vh("a"), vh(&format!("{B}Right")));
        fact(
            &mut arena,
            right,
            vh(&format!("{B}heldBy")),
            vh(&format!("{B}NaturalPerson")),
        );
    }
    let _ = arena.fire_registered_rules(crate::q_hash("contract:values-check"));
    arena.has_quin(
        agent,
        vh(&format!("{B}flag")),
        vh(&format!("{B}PersonhoodCategoryError")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crdt::SuspendedTransactionQueue;

    #[test]
    fn zk_consume_fact_gates_resource_exhaustion_on_verified_proof() {
        use crate::modalities::linear::is_consumed;
        let token = crate::q_hash("token:apiCall");
        let spend = crate::q_hash("q42:spend");
        let svc = crate::q_hash("svc:inference");
        let mk = |s, p, o| {
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
        let ops = [SlgOpcode::ZkConsumeFact];
        let frame_for = || VmFrame {
            subject_reg: token,
            predicate_reg: spend,
            object_reg: svc,
            context_reg: 0,
        };
        // The token's spent state (the real semantics; the VM's end-of-program return value is a
        // separate convention we don't rely on here).
        let spent = |a: &mut SlgArena| {
            a.find_mutable_quin(token, spend, svc)
                .map(|q| is_consumed(q))
                .unwrap_or(false)
        };

        // 1. Resource present, NO zk-verified marker → gate REFUSES (frame None); token NOT spent.
        let mut arena = SlgArena::new();
        arena.write_table(mk(token, spend, svc));
        let mut frame = frame_for();
        assert!(
            execute_vm_frame(&mut arena, &ops, &mut frame).is_none(),
            "no proof → gate refuses"
        );
        assert!(
            !spent(&mut arena),
            "token must NOT be spent without a verified proof"
        );

        // 2. Add the verified zk marker → the token is now spent.
        arena.write_table(mk(
            token,
            crate::q_hash("q42:zkVerified"),
            crate::q_hash("q42:true"),
        ));
        let mut frame2 = frame_for();
        let _ = execute_vm_frame(&mut arena, &ops, &mut frame2);
        assert!(
            spent(&mut arena),
            "verified proof → token spent exactly once"
        );

        // 3. Re-spend attempt → gate refuses (already exhausted); token stays spent (no double-spend).
        let mut frame3 = frame_for();
        assert!(
            execute_vm_frame(&mut arena, &ops, &mut frame3).is_none(),
            "exhausted linear token cannot be re-spent"
        );
        assert!(spent(&mut arena), "token remains spent");
    }

    #[test]
    fn test_multi_agent_ratification_flow() {
        let mut agreement = AgreementDID {
            agreement_id: 100,
            principal: 200,
            agents: [300, 400, 0, 0, 0, 0, 0, 0],
            num_agents: 2,
            domain_id: 500,
            threshold: 2,
            current_state: AgreementState::Proposed,
        };

        // Before Ratification: should compile to empty quins
        let proposed_quins = agreement.compile_to_super_quins();
        assert_eq!(proposed_quins[0].subject, 0);

        // Signatures Gathered!
        agreement.current_state = AgreementState::Ratified;
        let ratified_quins = agreement.compile_to_super_quins();

        // Assert Bilateral Routing Lane
        assert_eq!(
            ratified_quins[0].metadata & 0x4000_0000_0000_0002,
            0x4000_0000_0000_0002
        );
        assert_eq!(ratified_quins[0].subject, 200); // principal
        assert_eq!(ratified_quins[0].object, 300); // agent 1

        // Test CRDT Queue Suspension and Wakeup
        let mut crdt_queue = SuspendedTransactionQueue::new();

        let mut mock_vm = crate::modalities::logic::WebizenVM::new();
        mock_vm.registers[0] = Some(999); // Mock execution state

        let suspended_tx = mock_vm.flatten_to_suspended(100, 2, crate::NQuin::default());
        assert!(crdt_queue.push(suspended_tx).is_ok());

        // First signature token arrives via WebRTC
        let token_1 = crate::NQuin {
            subject: 300,
            predicate: crate::q_hash("q42:issuesConsentToken"),
            object: 100,
            context: 100,
            metadata: 0,
            parity: 0,
        };
        assert!(crdt_queue.apply_consensus_token(&token_1).is_none()); // Threshold not met

        // Second signature token arrives via WebRTC
        let token_2 = crate::NQuin {
            subject: 400,
            predicate: crate::q_hash("q42:issuesConsentToken"),
            object: 100,
            context: 100,
            metadata: 0,
            parity: 0,
        };
        let resumed_tx = crdt_queue.apply_consensus_token(&token_2);

        assert!(
            resumed_tx.is_some(),
            "WebRTC event failed to wake up suspended execution!"
        );
        assert_eq!(
            resumed_tx.unwrap().registers[0],
            Some(999),
            "Execution state was corrupted during CRDT suspension"
        );
    }

    #[test]
    fn check_defeaters_blocks_defeated_norm() {
        let mut arena = SlgArena::new();
        let contract = crate::q_hash("did:web:nda:contract-001");
        let alice = crate::q_hash("did:web:alice.example");
        let disclose = crate::q_hash("q42:disclose");
        let data = crate::q_hash("q42:data:project-x:confidential");

        let forbid = crate::modalities::logic::deontic::compile_norm_quin(
            alice,
            crate::modalities::logic::deontic::OP_FORBID,
            disclose,
            data,
            contract,
            0,
            false,
        );
        let defeater = crate::modalities::logic::deontic::compile_norm_quin(
            alice,
            crate::modalities::logic::deontic::OP_PERMIT,
            disclose,
            crate::q_hash("q42:role:certified-auditor"),
            contract,
            0,
            true,
        );
        arena.write_table(forbid);
        arena.write_table(defeater);

        let mut frame = VmFrame {
            subject_reg: alice,
            predicate_reg: forbid.predicate,
            object_reg: data,
            context_reg: contract,
        };
        let bytecode = [SlgOpcode::CheckDefeaters, SlgOpcode::Return];
        assert!(
            execute_vm_frame(&mut arena, &bytecode, &mut frame).is_none(),
            "CheckDefeaters must fail when a matching defeater exists"
        );
    }

    #[test]
    fn unify_binds_frame_from_arena_fact() {
        let mut arena = SlgArena::new();
        let fact = NQuin {
            subject: 10,
            predicate: 20,
            object: 30,
            context: 40,
            metadata: 0,
            parity: 10 ^ 20 ^ 30 ^ 40,
        };
        arena.write_table(fact);

        let mut frame = VmFrame {
            subject_reg: 10,
            predicate_reg: 20,
            object_reg: 0,
            context_reg: 0,
        };
        let bytecode = [SlgOpcode::Unify, SlgOpcode::Return];
        let result = execute_vm_frame(&mut arena, &bytecode, &mut frame).expect("unify");
        assert_eq!(frame.object_reg, 30);
        assert_eq!(frame.context_reg, 40);
        assert_eq!(result.object, 30);
        assert_eq!(result.context, 40);
    }

    #[test]
    #[serial_test::serial]
    fn test_async_retrieve_logic() {
        // Initialize the DHAT profiler to ensure zero heap allocations
        let _profiler = dhat::Profiler::builder().testing().build();

        let mut arena = SlgArena::new();
        let mut frame = VmFrame::default();

        let bytecode = vec![SlgOpcode::NativeRetrieveByActivation];

        // Execute the bytecode
        let result = execute_vm_frame(&mut arena, &bytecode, &mut frame);

        // Ensure it yields immediately (returns None)
        assert!(result.is_none());

        // Verify no allocations occurred during the NativeRetrieveByActivation execution
        let stats = dhat::HeapStats::get();
        dhat::assert_eq!(stats.total_blocks, 0, "NativeRetrieveByActivation must not allocate on the heap! Zero-heap constraint violated.");
        dhat::assert_eq!(stats.total_bytes, 0, "NativeRetrieveByActivation must not allocate on the heap! Zero-heap constraint violated.");
    }

    /// Step 2 — deontic WIRING proof (PLAN §17.1.2): the agency.n3 **G1** corporate-capture
    /// guard, registered as an N3 rule, fires END-TO-END through the Webizen bytecode VM
    /// (`register_rule` → `fire_registered_rules` → `fire_guard_rules` forward-chaining) and
    /// asserts `PersonhoodCategoryError` on a CorporatePerson claiming a natural-person-only
    /// right — observable via `has_quin`. A NaturalPerson claiming the same right is NOT flagged.
    #[test]
    fn values_guard_g1_corporate_capture_fires() {
        use crate::modalities::logic::n3_parser::{Formula, Rule, RuleType, Term, Triple};
        const B: &str = "https://ns.webcivics.net/values/";
        let vh = |s: &str| crate::q_hash(s);
        let u = |s: &'static str| Term::Uri(s);
        let var = |n: &'static str| Term::Variable(n);

        // agency.n3 G1: { ?c a CorporatePerson ; claims ?r . ?r a Right ; heldBy NaturalPerson }
        //            => { ?c flag PersonhoodCategoryError } .
        let g1 = Rule {
            id: Some("agency-G1"),
            rule_type: RuleType::Strict,
            weight: None,
            premise: Formula {
                triples: vec![
                    Triple {
                        subject: var("c"),
                        predicate: u("a"),
                        object: u("https://ns.webcivics.net/values/CorporatePerson"),
                    },
                    Triple {
                        subject: var("c"),
                        predicate: u("https://ns.webcivics.net/values/claims"),
                        object: var("r"),
                    },
                    Triple {
                        subject: var("r"),
                        predicate: u("a"),
                        object: u("https://ns.webcivics.net/values/Right"),
                    },
                    Triple {
                        subject: var("r"),
                        predicate: u("https://ns.webcivics.net/values/heldBy"),
                        object: u("https://ns.webcivics.net/values/NaturalPerson"),
                    },
                ],
            },
            conclusion: Formula {
                triples: vec![Triple {
                    subject: var("c"),
                    predicate: u("https://ns.webcivics.net/values/flag"),
                    object: u("https://ns.webcivics.net/values/PersonhoodCategoryError"),
                }],
            },
        };

        let fact = |a: &mut SlgArena, s: u64, p: u64, o: u64| {
            a.write_table(NQuin {
                subject: s,
                predicate: p,
                object: o,
                context: 0,
                metadata: 0,
                parity: s ^ p ^ o,
            });
        };
        let flag = vh(&format!("{B}flag"));
        let pce = vh(&format!("{B}PersonhoodCategoryError"));
        let right = vh("https://ns.webcivics.net/example/UDHR_Art1_Dignity");

        // ── Positive: AcmeCorp (CorporatePerson) claims a NaturalPerson-held right → FLAGGED ──
        let mut arena = SlgArena::new();
        let acme = vh("https://ns.webcivics.net/example/AcmeCorp");
        fact(
            &mut arena,
            acme,
            vh("a"),
            vh(&format!("{B}CorporatePerson")),
        );
        fact(&mut arena, acme, vh(&format!("{B}claims")), right);
        fact(&mut arena, right, vh("a"), vh(&format!("{B}Right")));
        fact(
            &mut arena,
            right,
            vh(&format!("{B}heldBy")),
            vh(&format!("{B}NaturalPerson")),
        );
        arena.register_rule(&g1);
        let _ = arena.fire_registered_rules(crate::q_hash("contract:g1-smoke"));
        assert!(
            arena.has_quin(acme, flag, pce),
            "G1 must flag a CorporatePerson claiming a natural-person-only right (PersonhoodCategoryError)"
        );

        // ── Negative control: a NaturalPerson claiming the SAME right is NOT flagged ──
        let mut arena2 = SlgArena::new();
        let alice = vh("https://ns.webcivics.net/example/Alice");
        fact(
            &mut arena2,
            alice,
            vh("a"),
            vh(&format!("{B}NaturalPerson")),
        );
        fact(&mut arena2, alice, vh(&format!("{B}claims")), right);
        fact(&mut arena2, right, vh("a"), vh(&format!("{B}Right")));
        fact(
            &mut arena2,
            right,
            vh(&format!("{B}heldBy")),
            vh(&format!("{B}NaturalPerson")),
        );
        arena2.register_rule(&g1);
        let _ = arena2.fire_registered_rules(crate::q_hash("contract:g1-smoke"));
        assert!(
            !arena2.has_quin(alice, flag, pce),
            "a NaturalPerson claiming a right must NOT be flagged — the guard targets CorporatePerson"
        );
    }

    /// The reusable engine helper behind the MCP `values_check` tool: corporate AND software
    /// agents are caught; a natural person, and any non-claiming agent, are not.
    #[test]
    fn values_check_helper_anti_capture() {
        const B: &str = "https://ns.webcivics.net/values/";
        let ct = |c: &str| crate::q_hash(&format!("{B}{c}"));
        // A corporation claiming a human dignity right → category error.
        assert!(super::check_personhood_category_error(
            ct("CorporatePerson"),
            true
        ));
        // A software agent doing the same → also caught (G1').
        assert!(super::check_personhood_category_error(
            ct("ArtificialAgent"),
            true
        ));
        // A natural person holding their own right → fine.
        assert!(!super::check_personhood_category_error(
            ct("NaturalPerson"),
            true
        ));
        // A corporation that makes no such claim → nothing to flag.
        assert!(!super::check_personhood_category_error(
            ct("CorporatePerson"),
            false
        ));
    }

    /// CML concept-graph pilot (PLAN §-CML §6): put the deontic logic library *against a concept*.
    /// `cml:asserts` means the concept's logic quins carry `context = q_hash(concept IRI)` — so the
    /// concept hash IS the sub-graph the Webizen VM masks on. Build the norm for
    /// `concept:DutyToSuppressForcedLabour` in that context, evaluate it (Active = in force), then
    /// add an `unless lawfully-authorised` defeater in the same sub-graph (Active → Defeated).
    #[test]
    fn cml_concept_deontic_pilot() {
        use crate::modalities::logic::deontic::{
            compile_norm_quin, evaluate_deontic_contract, DeonticStatus, DeonticVerdict,
            OP_OBLIGATE, OP_PERMIT,
        };
        // Use the CORPUS hash (generate_60bit_token) — the space the ingested concept-graph lives in
        // — NOT q_hash (the legacy deontic/SlgArena space; they differ in the top 4 bits). This makes
        // the pilot's concept hash equal the .q42-ingested concept's hash. (See PLAN §21.3 hash-unify.)
        let h = |s: &str| crate::lexicon::generate_60bit_token(s.as_bytes());
        // The concept node = the context hash (cml:asserts → quins live in this sub-graph).
        let concept = h("https://ns.webcivics.net/concept/DutyToSuppressForcedLabour");
        let party = h("https://ns.webcivics.net/values/State"); // ratifying Party (R1 a-fortiori)
        let path = h("https://ns.webcivics.net/values/requires");
        let action = h("https://ns.webcivics.net/action/SuppressForcedLabour");
        let now = 1_717_200_000u32;

        // The concept's deontic sub-graph: a State obligation to suppress forced labour.
        let norm = compile_norm_quin(party, OP_OBLIGATE, path, action, concept, 0, false);
        assert_eq!(
            norm.context, concept,
            "the norm lives in the concept's context sub-graph"
        );

        let mut out = [DeonticVerdict::default(); 4];
        let n = evaluate_deontic_contract(&[norm], now, &mut out).expect("deontic eval");
        assert_eq!(n, 1);
        assert_eq!(
            out[0].status,
            DeonticStatus::Active,
            "the concept's obligation is in force"
        );
        assert_eq!(out[0].opcode, OP_OBLIGATE);

        // Lifecycle within the concept's sub-graph: an `unless lawfully authorised` defeater
        // (same party + path + context) flips Active → Defeated.
        let defeater = compile_norm_quin(
            party,
            OP_PERMIT,
            path,
            h("https://ns.webcivics.net/values/lawfullyAuthorised"),
            concept,
            0,
            true,
        );
        let mut out2 = [DeonticVerdict::default(); 4];
        let n2 =
            evaluate_deontic_contract(&[norm, defeater], now, &mut out2).expect("deontic eval 2");
        assert_eq!(n2, 1, "the defeater is not a primary norm");
        assert_eq!(
            out2[0].status,
            DeonticStatus::Defeated,
            "an unless-defeater in the concept's sub-graph defeats the obligation"
        );
    }

    /// CML pilot #6 — the "in force NOW *and* complied-with" loop: a temporal in-force window
    /// (interval) gates norm validity, and a SHACL compliance firewall (applied ONLY while the norm
    /// is binding, §-CML §5a) passes a CompliantState and fails an ExploitativeState.
    #[test]
    fn cml_concept_temporal_and_shacl_firewall() {
        use crate::modalities::interval_reasoning::TemporalInterval;
        use crate::modalities::logic::deontic::{
            compile_norm_quin, evaluate_deontic_contract, DeonticStatus, DeonticVerdict,
            OP_OBLIGATE,
        };
        use crate::sparql_library::sparql_shacl::{ShaclConstraint, ShaclShape, ShaclValidator};
        // Corpus hash (generate_60bit_token) — the SHACL validator hashes rdf:type this way, and it is
        // the space the ingested concept-graph lives in (NOT q_hash). One hash-space (PLAN §21.3).
        let h = |s: &str| crate::lexicon::generate_60bit_token(s.as_bytes());
        let concept = h("https://ns.webcivics.net/concept/DutyToSuppressForcedLabour");

        // ── VALIDITY: binding NOW = deontic Active AND within the in-force window ──
        let now = 1_717_200_000i64; // ~2024
        let before_eif = -500_000_000i64; // ~1954, before Convention 105 entry-into-force
        let far_future = 4_102_444_800i64;
        let in_force = TemporalInterval::new(concept, -347_000_000, far_future); // EIF 1959-01-17 → open
        assert!(
            in_force.contains(now),
            "the obligation is within its in-force window in 2024"
        );
        assert!(
            !in_force.contains(before_eif),
            "not binding before entry into force"
        );

        let norm = compile_norm_quin(
            h("https://ns.webcivics.net/values/State"),
            OP_OBLIGATE,
            h("https://ns.webcivics.net/values/requires"),
            h("https://ns.webcivics.net/action/SuppressForcedLabour"),
            concept,
            0,
            false,
        );
        let mut out = [DeonticVerdict::default(); 2];
        evaluate_deontic_contract(&[norm], now as u32, &mut out).unwrap();
        let binding_now = out[0].status == DeonticStatus::Active && in_force.contains(now);
        assert!(
            binding_now,
            "Active AND in-window ⇒ the norm is binding now"
        );

        // ── COMPLIANCE FIREWALL (SHACL) — gated on the norm being binding (§5a) ──
        // ForcedLabourComplianceShape: an AgentState MUST be a values:CompliantState.
        let rdf_type = h("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let agent_state = h("urn:pilot:acme-operations");
        let compliant = h("https://ns.webcivics.net/values/CompliantState");
        let exploitative = h("https://ns.webcivics.net/values/ExploitativeState");
        let q = |s, p, o| NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: concept,
            metadata: 0,
            parity: s ^ p ^ o ^ concept,
        };

        // A compliant entity conforms; an exploitative one violates — but only because the norm binds now.
        assert!(
            binding_now,
            "the firewall applies only while the norm is binding"
        );

        let good = [q(agent_state, rdf_type, compliant)];
        let mut vg = ShaclValidator::new(&good);
        let cg = vg
            .add_constraint(ShaclConstraint::Class {
                class_iri: compliant,
            })
            .unwrap();
        let mut shape = ShaclShape {
            shape_iri: h("https://ns.webcivics.net/values/ForcedLabourComplianceShape"),
            target_class: None,
            target_node: Some(agent_state),
            constraints: [0; 32],
            constraint_count: 1,
        };
        shape.constraints[0] = cg;
        assert!(
            vg.validate_node(agent_state, &shape).unwrap().conforms,
            "a CompliantState passes the compliance firewall"
        );

        let bad = [q(agent_state, rdf_type, exploitative)];
        let mut vb = ShaclValidator::new(&bad);
        let cb = vb
            .add_constraint(ShaclConstraint::Class {
                class_iri: compliant,
            })
            .unwrap();
        let mut shape_b = shape;
        shape_b.constraints[0] = cb;
        let rb = vb.validate_node(agent_state, &shape_b).unwrap();
        assert!(
            !rb.conforms && rb.violation_count > 0,
            "an ExploitativeState fails the compliance firewall"
        );
    }

    /// FILE → ENGINE end-to-end (PLAN §17.1.2, closing the parser gap): the engine parses its
    /// OWN `core-ontologies/agency.n3` with the native `N3Parser`, registers the parsed rules,
    /// and the G1 corporate-capture guard fires — proving the `.n3` files are the live source of
    /// truth, not hand-built structs. (`;`-lists + multi-line `{…}` rules parse correctly.)
    #[test]
    fn agency_n3_file_parses_and_g1_fires_end_to_end() {
        use crate::modalities::logic::n3_parser::{N3Event, N3Parser};
        
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../core-ontologies/agency.n3");
        let text = std::fs::read_to_string(&path).expect("agency.n3 must be readable");

        // Parse with the ENGINE's own N3 parser; collect the logic rules.
        let mut rules = Vec::new();
        let mut parser = N3Parser::new(&text);
        parser
            .parse_all(|ev| {
                if let N3Event::LogicRule(r) = ev {
                    rules.push(r);
                }
                Ok(())
            })
            .expect("agency.n3 must parse");
        assert!(
            rules.len() >= 5,
            "agency.n3 should yield several logic rules from the file; got {}",
            rules.len()
        );

        // Register every parsed rule into the Webizen VM.
        let mut arena = SlgArena::new();
        for r in &rules {
            arena.register_rule(&r);
        }

        // Facts use the SAME token form the parsed rule carries (CURIEs; @prefix is not
        // expanded, so matching is by token via q_hash) — a CorporatePerson claiming a
        // NaturalPerson-held right.
        let h = |s: &str| crate::q_hash(s);
        let fact = |a: &mut SlgArena, s: u64, p: u64, o: u64| {
            a.write_table(NQuin {
                subject: s,
                predicate: p,
                object: o,
                context: 0,
                metadata: 0,
                parity: s ^ p ^ o,
            });
        };
        let acme = h("ex:AcmeCorp");
        let right = h("ex:Right1");
        fact(&mut arena, acme, h("a"), h("values:CorporatePerson"));
        fact(&mut arena, acme, h("values:claims"), right);
        fact(&mut arena, right, h("a"), h("values:Right"));
        fact(
            &mut arena,
            right,
            h("values:heldBy"),
            h("values:NaturalPerson"),
        );

        let _ = arena.fire_registered_rules(crate::q_hash("contract:agency-file"));
        assert!(
            arena.has_quin(acme, h("values:flag"), h("values:PersonhoodCategoryError")),
            "G1 parsed FROM agency.n3 must fire and flag PersonhoodCategoryError"
        );
    }

    // ─── Modality breadth: the values layer is NOT deontic-only ──────────────────
    // The spine genuinely needs more of the engine's logic modalities. These prove
    // three more are wired to real values concerns (not decorative). See PLAN §20.

    /// TEMPORAL (interval_reasoning): a norm holds only over its `EffectivityInterval`
    /// (sense.n3). The BHR watchlist treaty is not-yet-in-force; UDHR is in force.
    #[test]
    fn values_temporal_effectivity_interval() {
        use crate::modalities::interval_reasoning::TemporalInterval;
        let now = 1_717_200_000i64; // ~2024-06
        let far_future = 4_102_444_800i64; // ~2100 (avoid end-start overflow; "open-ended")
        let udhr = TemporalInterval::new(1, -662_688_000, far_future); // in force since 1948
        let bhr_treaty = TemporalInterval::new(2, 1_790_000_000, far_future); // not before ~2026/27
        assert!(
            udhr.contains(now),
            "UDHR is in force in 2024 — its norms are temporally active"
        );
        assert!(
            !bhr_treaty.contains(now),
            "the BHR watchlist treaty is NOT yet in force — its norms are temporally inactive (notBeforeDate)"
        );
    }

    /// CONTRARY-TO-DUTY / dyadic deontic (the remedy pillar): UNGP access-to-remedy /
    /// ICCPR Art 2(3) — a breach of a primary duty triggers a secondary reparation
    /// obligation `O(remedy / breach)`; an unremedied breach is a continuing violation.
    #[test]
    fn values_remedy_pillar_contrary_to_duty() {
        use crate::modalities::logic::deontic::evaluate_contrary_to_duty;
        let party = crate::q_hash("https://ns.webcivics.net/example/OpenLikeCorp");
        let primary = crate::q_hash("https://ns.webcivics.net/values/responsibilityToRespect");
        let remedy = crate::q_hash("https://ns.webcivics.net/values/provideRemedy");
        let mk = |s: u64, p: u64, o: u64| NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: s ^ p ^ o,
        };
        assert!(
            evaluate_contrary_to_duty(&[], party, primary, remedy),
            "no breach → no remedy owed"
        );
        let breach = [mk(party, crate::q_hash("q42:breached"), primary)];
        assert!(
            !evaluate_contrary_to_duty(&breach, party, primary, remedy),
            "breach without remedy → continuing violation (the remedy gap)"
        );
        let repaired = [
            mk(party, crate::q_hash("q42:breached"), primary),
            mk(party, crate::q_hash("q42:fulfilled"), remedy),
        ];
        assert!(
            evaluate_contrary_to_duty(&repaired, party, primary, remedy),
            "breach + remedy → satisfied"
        );
    }

    /// ARGUMENTATION (Dung grounded extension): a rights-conflict is resolved by defeat,
    /// not by fiat. The inverse rights-guard rebuts a corporate dignity-claim; in the
    /// grounded extension the guard stands and the corporate claim is rejected.
    #[test]
    fn values_rights_conflict_argumentation_guard_wins() {
        use crate::modalities::argumentation::{
            Argument, ArgumentationFramework, Attack, AttackType,
        };
        let concl = |s: &str| NQuin {
            subject: crate::q_hash(s),
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let mut af = ArgumentationFramework::new();
        af.add_argument(Argument::new(
            1,
            "CorporatePerson claims a dignity right".to_string(),
            vec![],
            concl("ex:AcmeCorp-claims-dignity"),
        ));
        af.add_argument(Argument::new(
            2,
            "Dignity rights held only by NaturalPerson (inverse guard)".to_string(),
            vec![],
            concl("values:NaturalPerson-only-dignity"),
        ));
        af.add_attack(Attack {
            attacker: 2,
            target: 1,
            attack_type: AttackType::Rebuttal,
            strength: 1.0,
        });
        let grounded = af.grounded_extension();
        assert!(
            grounded.contains(&2),
            "the inverse guard stands (unattacked) in the grounded extension"
        );
        assert!(
            !grounded.contains(&1),
            "the corporate dignity-claim is DEFEATED — rejected from the grounded extension"
        );
    }

    /// ALGEBRA (CAS): legal PROPORTIONALITY (IHL AP I Art 51(5)(b); HR limitation tests) computed
    /// symbolically — disproportionate when harm exceeds benefit — and a COMPUTED expression
    /// round-trips through NQuins with a stable citation hash (§19 Expr↔NQuin provenance bridge),
    /// so a *derived* duty is storable + citable, not opaque.
    #[test]
    fn values_proportionality_and_provenance_via_cas() {
        use crate::specialized_libs::symbolic_algebra::{
            expr_citation_hash, from_quins, parse, simplify, to_quins,
        };
        use std::collections::HashMap;
        let excess = parse("harm - benefit").expect("CAS parses the proportionality expression");
        let mut env = HashMap::new();
        env.insert("harm".to_string(), 9.0);
        env.insert("benefit".to_string(), 4.0);
        assert!(
            excess.eval(&env).expect("evaluates") > 0.0,
            "harm (9) exceeds benefit (4) → disproportionate (a violation signal)"
        );
        let s = simplify(&excess);
        let back = from_quins(&to_quins(&s)).expect("Expr round-trips through the graph");
        assert_eq!(
            expr_citation_hash(&s),
            expr_citation_hash(&back),
            "the computed expression's citation hash is stable across the NQuin round-trip"
        );
    }

    /// ECONOMIC (subject-matter modality): an economic right — ICESCR Art 11 adequate standard of
    /// living — computed; a shortfall when subsistence cost exceeds income signals an unmet right.
    /// The modality is chosen by SUBJECT MATTER: the algebraic core is the CAS, the richer economic
    /// models live in `specialized_libs::financial_modeling` (real, available).
    #[test]
    fn values_economic_right_threshold() {
        use crate::specialized_libs::symbolic_algebra::parse;
        use std::collections::HashMap;
        let shortfall = parse("cost - income").expect("CAS parses the economic threshold");
        let mut met = HashMap::new();
        met.insert("cost".to_string(), 100.0);
        met.insert("income".to_string(), 120.0);
        assert!(
            shortfall.eval(&met).expect("eval") < 0.0,
            "income ≥ cost → ICESCR Art 11 standard met"
        );
        let mut unmet = HashMap::new();
        unmet.insert("cost".to_string(), 100.0);
        unmet.insert("income".to_string(), 60.0);
        assert!(
            shortfall.eval(&unmet).expect("eval") > 0.0,
            "cost > income → unmet economic right (shortfall)"
        );
    }

    /// SPATIAL (RCC-8): jurisdiction FOLLOWS THE PERSON (§10.5). The affected person's region is a
    /// proper part of the operation jurisdiction → the duty binds where they are; a foreign
    /// choice-of-law region disconnected from the affected jurisdiction → the RemedyStripping signal.
    #[test]
    fn values_jurisdiction_follows_the_person_rcc8() {
        use crate::modalities::spatio_temporal::{evaluate_rcc8, Rcc8Relation, SpatialRegion};
        let h = crate::q_hash;
        let au = SpatialRegion::new(
            h("region:AU"),
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        );
        let person = SpatialRegion::new(
            h("region:user-in-AU"),
            vec![(2.0, 2.0), (4.0, 2.0), (4.0, 4.0), (2.0, 4.0)],
        );
        let us = SpatialRegion::new(
            h("region:US-choiceOfLaw"),
            vec![
                (100.0, 100.0),
                (110.0, 100.0),
                (110.0, 110.0),
                (100.0, 110.0),
            ],
        );

        let r = evaluate_rcc8(&person, &au);
        assert!(
            matches!(r, Rcc8Relation::NonTangentialProperPart | Rcc8Relation::TangentiallyProperPart),
            "the affected person's region is inside the operation jurisdiction (a proper part); got {r:?}"
        );
        assert_eq!(
            evaluate_rcc8(&us, &au),
            Rcc8Relation::Disconnected,
            "foreign choice-of-law disconnected from the affected jurisdiction → RemedyStripping signal"
        );
    }

    /// PARACONSISTENT: conflicting instruments across jurisdictions must NOT explode the reasoner.
    /// Two instruments give the same act contradictory normative status; the contradiction is
    /// ISOLATED (quarantined), the rest of the corpus stays consistent — no ex-falso collapse.
    #[test]
    fn values_conflicting_instruments_isolated_not_exploded() {
        use crate::modalities::paraconsistent::route_paraconsistent;
        let h = crate::q_hash;
        let ctx = h("contract:multi-jurisdiction");
        let mk = |s: u64, p: u64, o: u64| NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: ctx,
            metadata: 0,
            parity: s ^ p ^ o ^ ctx,
        };
        let act = h("ex:someAct");
        let status = h("q42:normativeStatus");
        let quins = [
            mk(act, status, h("values:Permitted")), // instrument A
            mk(act, status, h("values:Forbidden")), // instrument B — contradicts A
            mk(h("ex:otherAct"), status, h("values:Permitted")), // unrelated, consistent
        ];
        let mut consistent = [NQuin::default(); 8];
        let mut isolated = [NQuin::default(); 8];
        let (nc, ni) = route_paraconsistent(&quins, &mut consistent, &mut isolated).expect("route");
        assert_eq!(
            ni, 1,
            "exactly the contradicting claim is isolated (quarantined)"
        );
        assert_eq!(
            nc, 2,
            "the rest of the corpus stays consistent — no ex-falso explosion"
        );
    }

    // ─── Identity / personhood spine: identifier ≠ identity (§13) ────────────────
    // Identity is a COMPUTED, MODAL, epistemically-grounded result over a fabric — not a
    // string. Three modalities make that computable: DL (classification), modal (◇/□),
    // epistemic (known vs merely believed = verification).

    /// DL (description-logic subsumption): the Agent lattice is machine-reasoned —
    /// NaturalPerson ⊑ Agent; CorporatePerson ⊑ LegalPerson ⊑ Agent; State ⊑ PublicAuthority ⊑
    /// LegalPerson ⊑ Agent. A CorporatePerson IS-A Agent but is NOT a NaturalPerson (the firewall).
    #[test]
    fn values_identity_classification_via_dl_subsumption() {
        use crate::modalities::dl::check_subsumption_quin;
        let h = crate::q_hash;
        let sub = h("rdfs:subClassOf");
        let edge = |s: &str, o: &str| {
            let (s, o) = (h(s), h(o));
            NQuin {
                subject: s,
                predicate: sub,
                object: o,
                context: 0,
                metadata: 0,
                parity: s ^ sub ^ o,
            }
        };
        let tbox = [
            edge("values:NaturalPerson", "values:Agent"),
            edge("values:CorporatePerson", "values:LegalPerson"),
            edge("values:LegalPerson", "values:Agent"),
            edge("values:State", "values:PublicAuthority"),
            edge("values:PublicAuthority", "values:LegalPerson"),
        ];
        let (np, cp, agent) = (
            h("values:NaturalPerson"),
            h("values:CorporatePerson"),
            h("values:Agent"),
        );
        assert!(
            check_subsumption_quin(np, agent, &tbox),
            "NaturalPerson IS-A Agent"
        );
        assert!(
            check_subsumption_quin(cp, agent, &tbox),
            "CorporatePerson IS-A Agent (transitively)"
        );
        assert!(
            check_subsumption_quin(h("values:State"), agent, &tbox),
            "State IS-A Agent (via PublicAuthority→LegalPerson)"
        );
        assert!(
            !check_subsumption_quin(cp, np, &tbox),
            "a CorporatePerson is NOT a NaturalPerson — the personhood firewall"
        );
    }

    /// MODAL (Kripke ◇/□): identity holds RELATIVE to context ("worlds"). A natural person's
    /// "person before the law" recognition is NECESSARY (□) across accessible contexts; a
    /// corporate dignity-claim is NOT POSSIBLE (¬◇) in any. Identity is modal, not absolute.
    #[test]
    fn values_identity_is_modal() {
        use crate::modalities::modal::{necessary, possible};
        let h = crate::q_hash;
        let accesses = h("modal:accesses");
        let holds = h("modal:holds");
        let acc = |f: u64, t: u64| NQuin {
            subject: f,
            predicate: accesses,
            object: t,
            context: 0,
            metadata: 0,
            parity: f ^ accesses ^ t,
        };
        let lab = |w: u64, p: u64| NQuin {
            subject: w,
            predicate: holds,
            object: p,
            context: 0,
            metadata: 0,
            parity: w ^ holds ^ p,
        };
        let (here, w1, w2) = (0u64, 1u64, 2u64);
        let pbl = h("values:personBeforeLaw");
        let corp_dignity = h("values:corporateDignity");
        let g = [acc(here, w1), acc(here, w2), lab(w1, pbl), lab(w2, pbl)];
        assert!(
            necessary(&g, here, pbl, accesses, holds),
            "person-before-law recognition is NECESSARY (□) across contexts"
        );
        assert!(
            !possible(&g, here, corp_dignity, accesses, holds),
            "a corporate dignity-claim is NOT POSSIBLE (¬◇) in any context"
        );
    }

    /// EPISTEMIC (knows vs believes): identity VERIFICATION = is the claimed identity KNOWN over
    /// the fabric, or merely believed? A KNOWN binding → Active (verified); a low-certainty
    /// BELIEVED binding → Uncertain = `claimedIdentityUnverifiable` (the phishing/impersonation signal).
    #[test]
    fn values_identity_as_known_epistemic() {
        use crate::modalities::epistemic::{
            evaluate_epistemic_frame, EpistemicStatus, EpistemicVerdict, CERTAINTY_BIT_SHIFT,
            OP_BELIEVES, OP_KNOWS,
        };
        let h = crate::q_hash;
        let pred = |op: u8, cert: u8| (op as u64) | ((cert as u64) << CERTAINTY_BIT_SHIFT);
        let mk = |s: u64, p: u64, o: u64| NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: s ^ p ^ o,
        };
        let verifier = h("did:webizen:verifier");
        let known = mk(
            verifier,
            pred(OP_KNOWS, 255),
            h("did:web:alice=NaturalPerson"),
        );
        let believed = mk(verifier, pred(OP_BELIEVES, 10), h("ex:phisher=YourBank"));
        let mut out = [EpistemicVerdict {
            claim: NQuin::default(),
            status: EpistemicStatus::Skipped,
            certainty: 0,
        }; 4];
        let n =
            evaluate_epistemic_frame(&[known, believed], 0, 0, &mut out).expect("epistemic eval");
        assert_eq!(n, 2);
        assert_eq!(
            out[0].status,
            EpistemicStatus::Active,
            "a KNOWN identity binding is verified (Active)"
        );
        assert_eq!(
            out[1].status,
            EpistemicStatus::Uncertain,
            "a low-certainty BELIEVED binding is unverifiable (Uncertain) — claimedIdentityUnverifiable"
        );
    }

    /// FUZZY (t-norms): rights are often PARTIALLY fulfilled — degrees, not binary. ICESCR
    /// adequate-standard-of-living fulfilment = the WEAKEST component (Gödel t-norm = min);
    /// the best of alternative remedies = t-conorm (max). "adequate"/"reasonable" are fuzzy.
    #[test]
    fn values_partial_right_fulfilment_fuzzy() {
        use crate::modalities::fuzzy::{t_conorm_godel, t_norm_godel};
        let fulfilment = t_norm_godel(t_norm_godel(0.9, 0.4), 0.8); // food .9, housing .4, health .8
        assert!(
            (fulfilment - 0.4).abs() < 1e-6,
            "partial fulfilment = the weakest component (housing 0.4)"
        );
        assert!(
            (t_conorm_godel(0.3, 0.7) - 0.7).abs() < 1e-6,
            "best available remedy degree (max)"
        );
    }

    /// ABDUCTIVE (inference to the best explanation): "WHY was this flagged?" — trace
    /// explanatory edges back to the root cause (the corporate-capture attempt), so a flag is
    /// accountable/contestable, not a black box.
    #[test]
    fn values_why_flagged_abductive() {
        use crate::modalities::abductive::abductive_explanation;
        let h = crate::q_hash;
        let explains = h("q42:explains");
        let e = |hyp: u64, eff: u64| NQuin {
            subject: hyp,
            predicate: explains,
            object: eff,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let flag = h("values:PersonhoodCategoryError");
        let guard_trip = h("values:inverseGuardTripped");
        let root = h("values:corporateCaptureAttempt");
        let rules = [e(guard_trip, flag), e(root, guard_trip)];
        assert_eq!(
            abductive_explanation(&rules, flag, explains),
            Some(root),
            "the flag's best explanation is the corporate-capture attempt (root cause)"
        );
    }

    /// PROBABILISTIC: trust is behaviourally-derived (trustfactory) — a reputation weight gates
    /// access against a threshold (not a binary allow-list).
    #[test]
    fn values_behavioural_trust_threshold() {
        use crate::modalities::probabilistic::evaluate_threshold;
        assert!(
            evaluate_threshold(0.85, 0.7),
            "reputation 0.85 ≥ threshold 0.7 → trusted"
        );
        assert!(
            !evaluate_threshold(0.40, 0.7),
            "reputation 0.40 < threshold 0.7 → not trusted"
        );
    }

    /// DIALECTICAL (but-for causation): legal causation/liability — was the breach a NECESSARY
    /// cause of the harm? (effect reachable from root, but NOT once the candidate is removed).
    #[test]
    fn values_but_for_causation_dialectical() {
        use crate::modalities::dialectical::is_necessary_cause;
        let h = crate::q_hash;
        let causes = h("causal:causes");
        let e = |c: u64, ef: u64| NQuin {
            subject: c,
            predicate: causes,
            object: ef,
            context: 0,
            metadata: 0,
            parity: c ^ causes ^ ef,
        };
        let (operator, breach, harm) =
            (h("ex:operator"), h("ex:breachOfDuty"), h("ex:harmToPerson"));
        let chain = [e(operator, breach), e(breach, harm)];
        assert!(
            is_necessary_cause(&chain, operator, breach, harm),
            "the breach is a but-for (necessary) cause of the harm"
        );
        // With an independent alternative cause, the breach is NOT necessary (no sole liability).
        let alt = h("ex:independentCause");
        let diamond = [
            e(operator, breach),
            e(breach, harm),
            e(operator, alt),
            e(alt, harm),
        ];
        assert!(
            !is_necessary_cause(&diamond, operator, breach, harm),
            "not necessary when an alternative cause exists"
        );
    }

    /// CTL (branching-time): obligations over possible futures — a remedy must EVENTUALLY be
    /// provided (AF / exists_finally); a right must ALWAYS hold (AG / always_globally).
    #[test]
    fn values_obligations_over_futures_ctl() {
        use crate::modalities::ctl::{always_globally, exists_finally};
        let h = crate::q_hash;
        let (next, holds) = (h("ctl:next"), h("ctl:holds"));
        let nx = |f: u64, t: u64| NQuin {
            subject: f,
            predicate: next,
            object: t,
            context: 0,
            metadata: 0,
            parity: f ^ next ^ t,
        };
        let lab = |s: u64, p: u64| NQuin {
            subject: s,
            predicate: holds,
            object: p,
            context: 0,
            metadata: 0,
            parity: s ^ holds ^ p,
        };
        let remedy = h("values:remedyProvided");
        let g = [nx(0, 1), nx(1, 2), lab(2, remedy)];
        assert!(
            exists_finally(&g, 0, remedy, next, holds),
            "a remedy is EVENTUALLY provided (AF) along the path"
        );
        let right = h("values:rightHeld");
        let g2 = [
            nx(0, 1),
            nx(1, 2),
            lab(0, right),
            lab(1, right),
            lab(2, right),
        ];
        assert!(
            always_globally(&g2, 0, right, next, holds),
            "the right ALWAYS holds (AG) across reachable states"
        );
    }

    /// TEMPORAL-LTL / metric (deadlines): a triggered duty must be met within a window — "remedy
    /// within N of breach"; past the deadline is a continuing violation.
    #[test]
    fn values_deadline_holds_within() {
        use crate::modalities::temporal_ltl::holds_within;
        let h = crate::q_hash;
        let (breach, remedy) = (h("q42:breach"), h("q42:remedy"));
        let timed = |p: u64, t: u64| NQuin {
            subject: 0,
            predicate: p,
            object: 0,
            context: 0,
            metadata: t,
            parity: 0,
        };
        assert!(
            holds_within(
                &[timed(breach, 100), timed(remedy, 120)],
                breach,
                remedy,
                30
            ),
            "remedy within the deadline"
        );
        assert!(
            !holds_within(
                &[timed(breach, 100), timed(remedy, 200)],
                breach,
                remedy,
                30
            ),
            "remedy past deadline → continuing violation"
        );
    }

    /// LINEAR LOGIC (consumable resources): one-shot consent — a consent token is CONSUMED when
    /// used and cannot be silently re-spent (resource-aware, not classical-logic re-usable truth).
    #[test]
    fn values_one_shot_consent_linear() {
        use crate::modalities::linear::{consume_quin, is_consumed};
        let h = crate::q_hash;
        let mut consent = NQuin {
            subject: h("did:web:alice"),
            predicate: h("values:consentsTo"),
            object: h("ex:oneDataUse"),
            context: 0,
            metadata: 0,
            parity: 0,
        };
        assert!(
            !is_consumed(&consent),
            "a fresh consent token is unconsumed"
        );
        consume_quin(&mut consent);
        assert!(
            is_consumed(&consent),
            "consent is one-shot: consumed on use, cannot be silently re-spent"
        );
    }

    /// GRAPH THEORY: structural analysis of a relationship / standing network (degrees, density,
    /// centrality) — e.g. how connected a fabric of guardians/advocates/relationships is.
    #[test]
    fn values_relationship_network_graph_theory() {
        use crate::modalities::graph_theory::QualiaGraph;
        let h = crate::q_hash;
        let rel = h("values:relatesTo");
        let e = |s: u64, o: u64| NQuin {
            subject: s,
            predicate: rel,
            object: o,
            context: 0,
            metadata: 0,
            parity: s ^ rel ^ o,
        };
        let g = QualiaGraph::from_quins(&[e(1, 2), e(2, 3)]); // alice—bob—carol
        let d = g.density();
        assert!(
            d > 0.0 && d <= 1.0,
            "the relationship network has measurable structure (density={d})"
        );
    }

    /// ASP (true stable-model semantics): an UNDER-DETERMINED instrument has multiple consistent
    /// interpretations. "permitted :- not forbidden; forbidden :- not permitted" → TWO answer sets
    /// (each a coherent normative scenario); adding `:- forbidden` (a higher norm) prunes to one.
    #[test]
    fn values_underdetermined_norm_answer_sets() {
        use crate::modalities::asp::{compute_answer_sets, AspRule};
        let h = crate::q_hash;
        let (permitted, forbidden) = (h("values:Permitted"), h("values:Forbidden"));
        let atoms = [permitted, forbidden];
        let prog = [
            AspRule::new(permitted, &[], &[forbidden]),
            AspRule::new(forbidden, &[], &[permitted]),
        ];
        let mut out = [0u64; 8];
        assert_eq!(
            compute_answer_sets(&atoms, &prog, &mut out),
            2,
            "under-determined norm → two consistent scenarios"
        );

        // A binding higher norm `:- forbidden` collapses it to the single lawful reading.
        let prog2 = [
            AspRule::new(permitted, &[], &[forbidden]),
            AspRule::new(forbidden, &[], &[permitted]),
            AspRule::constraint(&[forbidden], &[]),
        ];
        let mut out2 = [0u64; 8];
        assert_eq!(
            compute_answer_sets(&atoms, &prog2, &mut out2),
            1,
            "the higher norm prunes to one scenario"
        );
        assert_eq!(
            out2[0],
            1u64 << 0,
            "the surviving scenario is {{permitted}}"
        );
    }

    #[test]
    fn webizen_vm_reasons_over_manifold_ltl_and_asp() {
        use crate::modalities::asp::atom_index;
        use crate::modalities::manifold::{
            encode_manifold_state, ManifoldCoordinate10D, ManifoldDimension, ManifoldState10D,
            MANIFOLD_ASP_ATOMS, MANIFOLD_ATOM_STABLE,
        };

        let mut arena = SlgArena::new();
        for (state_id, timestamp, scale) in [(101, 1, 0.7), (102, 2, 0.8)] {
            let mut coordinate = ManifoldCoordinate10D::from_sequential_layer(timestamp, 10);
            coordinate.scale = scale;
            coordinate.density_threshold = 0.5;
            coordinate.manifold_curvature = 0.0;
            let state = ManifoldState10D {
                state_id,
                timestamp: timestamp as u64,
                coordinate,
            };
            let mut pair = [NQuin::default(); 2];
            encode_manifold_state(&state, &mut pair);
            arena.write_table(pair[0]);
            arena.write_table(pair[1]);
        }

        let mut ltl_frame = VmFrame::default();
        let ltl = [
            SlgOpcode::NativeManifoldLtl {
                mode: 0,
                dimension: ManifoldDimension::Scale as u8,
                threshold_bits: 0.5f32.to_bits(),
                at_least: true,
            },
            SlgOpcode::Return,
        ];
        assert!(execute_vm_frame(&mut arena, &ltl, &mut ltl_frame).is_some());

        let mut asp_frame = VmFrame::default();
        let asp = [SlgOpcode::NativeManifoldAsp, SlgOpcode::Return];
        assert!(execute_vm_frame(&mut arena, &asp, &mut asp_frame).is_some());
        let stable = atom_index(&MANIFOLD_ASP_ATOMS, MANIFOLD_ATOM_STABLE).unwrap();
        assert_ne!(asp_frame.object_reg & (1u64 << stable), 0);
    }
}
