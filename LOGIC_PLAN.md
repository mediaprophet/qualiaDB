# Logic Modalities — Build Plan

Goal: every logic modality the rights/governance ontology needs is (a) a real
zero-heap evaluator over `NQuin`s, (b) wired to a Webizen VM `Native*` opcode with
gate semantics, (c) covered by a unit test + a live-lane VM test, AND (d) carries a
**SHACL extension** (a shape constraining its configuration/structure, in the
`webizen.org/q42#` namespace, mirroring `core_modalities_shacl.rs`).

Discipline (the agent-honesty mandate): no modality is "done" until its real
round-trip test passes and the zero-heap measurement is green. Every claim here is
falsifiable by `cargo test`.

## Pattern (per modality)
1. `modalities/<name>.rs` — evaluator over `&[NQuin]` / frame regs; fixed stack
   buffers only (zero-heap); `#[cfg(test)]` unit test.
2. `webizen.rs` — `Native<Name>` opcode + handler (gate: violated → `return None`),
   reusing `[NQuin; 512]` stack scratch.
3. `tests/modalities_active.rs` — live-lane `[op, Return]` test (pass + fail).
4. `tests/zero_heap_modalities.rs` — assert 0 allocations.
5. SHACL: a `q42:<Name>ConfigurationShape` in `shapes/logic-modalities.shacl.ttl`
   + `logic_modalities_shacl.rs::get_logic_modalities_shacl_ttl()`.

## Already wired + tested (12) — add SHACL shapes
deontic, epistemic, linear, asp, paraconsistent, dialectical, defeasible, LTL,
Allen-interval, probabilistic, DL-subsumption, argumentation.

## New modalities to build

### Batch 1 — rights-load-bearing
1. **Metric/timed temporal (MTL)** — `NativeMtlWithin(u32)`. Trace quins carry a
   timestamp (`metadata`); given a trigger proposition (frame.predicate) and a
   target (frame.object), the target must occur within N time-units of the trigger.
   Deadlines: "remedy within 30 days", "obligation expires at majority".
2. **Contrary-to-duty / dyadic deontic (CTD)** — `NativeContraryToDuty`. O(B/A):
   given a primary violation A is present in the arena, the secondary (reparation)
   obligation B must also be present; else flag. The remedy-after-breach logic
   (Geneva/reparation instruments).
3. **Causal necessity (but-for)** — `NativeCausalNecessary`. Over cause→effect edge
   quins: a candidate cause (frame.subject) is necessary for an effect
   (frame.object) iff removing it disconnects all paths cause→effect. Attribution /
   liability (A-platform/A-sanction chain). Bounded BFS, zero-heap.

### Batch 2 — non-monotonic & truth-degree
4. **Abductive** — `NativeAbduce`. Inference to best explanation: given observation
   (frame.object) and rule edges (hypothesis→observation), an explanatory hypothesis
   exists. Backward search, zero-heap.
5. **Default logic (Reiter)** — `NativeDefaultApplies`. Default `P : Q / Q` applies
   iff prerequisite P holds and the negation of justification Q is absent
   (consistency = closed-world absence). Negation-as-failure gate.
6. **Fuzzy / many-valued (Łukasiewicz/Gödel)** — `NativeFuzzyThreshold(u32)`.
   Proposition truth-degree (f32 in `metadata`); conjunction via a t-norm; gate on
   degree ≥ threshold. Degrees of (partial) satisfaction.

### Batch 3 — temporal/modal breadth
7. **Branching-time CTL** — `NativeCtlExistsFinally` / `NativeCtlAlwaysGlobally`.
   Over state-transition quins: EF(target) = some path reaches target; AG(inv) = all
   reachable states satisfy inv. Bounded reachability, zero-heap.
8. **General modal (Kripke S4/S5)** — `NativeModalNecessary` / `NativeModalPossible`.
   Over accessibility quins (world→world) + valuation quins (world,prop): □φ holds in
   all accessible worlds; ◇φ in some. The shared modal substrate.

## SHACL extension layer (all 20 modalities)
`shapes/logic-modalities.shacl.ttl` + `logic_modalities_shacl.rs`: one
`q42:<Modality>ConfigurationShape` per modality constraining its parameters
(e.g. certainty ≤ 255, threshold ∈ [0,1], window > 0, trace length bounds) and the
predicate-opcode packing convention. Validated (rdflib) + a Rust test that the TTL
parses via the in-crate SHACL compiler.

## Status (updated as each lands)
- [x] Batch 1: MTL, CTD, causal — DONE (no duplicates: MTL→temporal_ltl::holds_within,
      CTD→deontic::evaluate_contrary_to_duty, causal→dialectical::is_necessary_cause;
      all zero-heap, modalities_active 19/19, zero_heap 1/1).
- [x] Batch 2: abductive, NAF/closed-world (default), fuzzy — DONE (abductive.rs + fuzzy.rs new; default→defeasible::holds_by_default NAF, NOT a defeasible duplicate). modalities_active 22/22.
- [x] Batch 3: CTL (branching-time, new ctl.rs) + general modal Kripke □/◇ (new modal.rs) — DONE. modalities_active 24/24, zero_heap 1/1.
- [ ] SHACL extensions (all modalities)

DUPLICATE-CHECK FINDINGS (check before each build): causal/counterfactual already
existed in `dialectical.rs` (counterfactual_query/find_causal_paths — heap); metric
temporal in `temporal_ltl.rs` (evaluate_lock_lease) + `deontic.rs` expiry; reparation
ontology in `remedy_reparation_annotated.ttl`. We EXTENDED these, didn't duplicate.

Sequencing: build evaluator+opcode+tests per modality, commit per batch; SHACL last
(it references the final opcode/predicate conventions). Every batch ends green
(`cargo test` for the touched crate) before moving on.
