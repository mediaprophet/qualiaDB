use super::*;

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
    /// Pre-parsed rules indexed by id; activated via `NativeRegisterRule`.
    #[cfg(feature = "alloc_buffers")]
    staged_rules: alloc::vec::Vec<CompiledRule>,
    #[cfg(not(feature = "alloc_buffers"))]
    staged_rules: std::vec::Vec<CompiledRule>,
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
        #[cfg(feature = "alloc_buffers")]
        let staged_rules = alloc::vec::Vec::new();
        #[cfg(not(feature = "alloc_buffers"))]
        let staged_rules = std::vec::Vec::new();

        Self {
            buffer,
            head_pointer: 0,
            recent_slots: [0; RECENT_SLOT_RING],
            recent_slot_head: 0,
            rule_registry,
            staged_rules,
        }
    }

    /// Registers a logical implication rule into the Webizen VM
    pub fn register_rule(&mut self, rule: &Rule<'_>) {
        vm_log!("🧠 Webizen registered new Compiled N3 Rule");
        self.rule_registry.push(compile_rule_to_zero_heap(rule));
    }

    /// Stage a parsed rule for later activation via `NativeRegisterRule`.
    /// Returns the rule id (`object_reg` value) used to activate it.
    pub fn stage_rule(&mut self, rule: &Rule<'_>) -> u64 {
        let id = self.staged_rules.len() as u64;
        self.staged_rules.push(compile_rule_to_zero_heap(rule));
        id
    }

    /// Activate a staged rule by id into the live rule registry.
    pub fn activate_staged_rule(&mut self, rule_id: u64) -> bool {
        let idx = rule_id as usize;
        if idx >= self.staged_rules.len() {
            return false;
        }
        self.rule_registry.push(self.staged_rules[idx].clone());
        true
    }

    pub fn rule_count(&self) -> usize {
        self.rule_registry.len()
    }

    pub fn staged_rule_count(&self) -> usize {
        self.staged_rules.len()
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
