# Handover — core-ontologies (implementation START HERE)

Planning phase complete (2026-06-21). This is the entry pointer; the full spec + decision log
is **`core-ontologies/PLAN.md`** (17 sections). Project memory: `project-values-credentials-ontologies`.

## State
- **Corpus**: 102+ instruments (OHCHR + ICRC Geneva I–IV + AP I–III + Commonwealth), all valid
  Turtle, 24 categories (`core-ontologies/INDEX.md`).
- **Spine ontologies** (8 files, all rdflib-validated):
  `selfhood.n3` (foundational person; human-not-thing) → `values.n3` (Agent lattice + R1/R2/R3 +
  SHACL) → `agency.n3` (personhood depth, agent attribution, accountability, jurisdiction,
  contract, standing, defeasible guards) → `sense.n3` (time/space sense + human-meaning-authority)
  → `tiering.n3` (governance vocab) → `policy.n3` (enforcement modes + harm/fraud categories) →
  `humanitarian-ict.n3` (the positive/prioritise pole) → `traces/personhood_agency.trace.n3`
  (6-case regression fixture).
- **Deontic evaluator EXISTS** in `webizen.rs` (`execute_vm_frame`, `NativeDeonticEval` →
  `evaluate_deontic_contract` → 64-byte `DeonticVerdict[]`; natively defeasible via
  `defeasible.rs`). Remaining deontic work = wiring `values.n3` in, not building an engine.
- 4 external reviews (Gemini/Grok/Codex) converged; corrections folded into PLAN.

## Do this next (PLAN §17)
1. **First slice (one verified session, no agents):** Webizen values-credential smoke test (§11.3) →
   deontic wiring (§5) → `validate_core_ontologies` gate + `build_index.py` gap report (§9.1).
2. **Then parallelise with agents (gate-checkable only):** remaining acquisition (§2), fix the
   2 under-segmented OHCHR files, CML-HTML/`.q42` layers (§5 step 2).
3. **Big dedicated passes (own sessions):** comprehensive sense (all-languages/all-media,
   browser+10d+protocol — §16), comprehensive selfhood, namespace standardisation, browser/policy
   integration, fraud + identity-verification, humanitarian-ICT discovery.

## Pin before specific work
- Predicate-packing convention (§9.2) before compiling sense/temporal rules.
- Namespace: ✅ DECIDED + migrated to `https://ns.webcivics.org/` (2026-06-21; qualia.id was
  unavailable). All `.n3` + generators + `owl.rs` updated and validated.

## Discipline (the lesson)
Compile-green ≠ works (the Antigravity ZK case compiled but its prove/verify test failed). Run the
actual round-trip; verify every agent claim against the tree. The smoke test + gate exist precisely
to catch overclaiming — that is why they come before agent parallelisation.
