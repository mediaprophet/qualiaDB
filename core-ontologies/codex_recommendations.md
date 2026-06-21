# Codex Recommendations for `core-ontologies/PLAN.md`

Reviewed: 2026-06-21
Pass: second review, after the updated plan incorporated the first Codex review.

## Summary

The updated plan is now much closer to the implementation reality. It correctly frames the
deontic issue as wiring into the existing Webizen Sentinel VM, not as a missing evaluator,
and it adopts the overlay, validation, gap-report, and "no mocks" recommendations from the
first pass.

The remaining recommendations are therefore narrower: turn the conceptual spine into
executable fixtures, pin a few modelling decisions before compiling rules, and move the
validation and wiring tasks earlier in the sequence. The corpus is already large enough;
the next win is an auditable end-to-end path.

## Verified Local State

- `webizen.rs` contains `register_rule`, `fire_registered_rules`, `execute_vm_frame`,
  `NativeDeonticEval`, and `NativeEpistemicEval`.
- `n3_compiler.rs` contains `compile_rule_to_opcodes` and calls
  `deontic::compile_n3_rule_to_norm`.
- `deontic.rs` contains `compile_n3_rule_to_norm` and `evaluate_deontic_contract`.
- `ingest.rs` and `mcp_server.rs` call the registered-rule path.
- `core-ontologies/sense.n3`, `core-ontologies/tiering.n3`,
  `core-ontologies/mutable/`, and `core-ontologies/overlays/` do not exist yet.
- The two under-segmented OHCHR files still contain `Download: PDF` boilerplate.

## Highest Priority Recommendations

1. Move validation and wiring earlier than the acquisition backlog.

   Section 7 still places "curation overlays + CML/q42 + deontic wiring" last. That is now
   the main sequencing mismatch. The wiring and validation gates should happen before more
   acquisition, because they define what "correctly acquired" means.

   Recommended near-term order:

   - `tiering.n3`
   - `sense.n3`
   - personhood, platform-agent, contract-capacity, and guardian terms in `values.n3`
   - native `validate_core_ontologies` gate
   - four/five-case Sentinel trace through the actual Webizen path
   - fix the two under-segmented OHCHR files
   - then resume acquisition

2. Add one executable "values Sentinel smoke test."

   The plan now names the correct engine path. The next step is to prove that this
   specific corpus uses it.

   The test should load a tiny fixture containing `values.n3` rules plus a few facts, then
   assert:

   - rules are parsed and registered
   - `fire_registered_rules` emits norms or executable opcodes
   - `NativeDeonticEval` or the equivalent compiled path runs
   - `values:violates` or a typed verdict is observable
   - `n3logic.rs::infer_logic_bindings` is not required for this path

   This is the best guard against a subtle failure mode: all engine pieces exist, but the
   values corpus never enters the live parse-compile-execute lane.

3. Convert Section 4.3 into canonical fixtures immediately.

   The updated plan's trace now has five cases. Treat them as regression fixtures, not
   just examples:

   - `LegalPerson` claims UDHR Article 1 dignity right: `PersonhoodCategoryError`
   - `NaturalPerson` claims the same right: pass
   - company claims ECHR Article 6 or property protection: pass only through curated overlay
   - autonomous agent acts with no guardian: ungrounded agency flag
   - platform AI harms a user in region A while ToS selects law B: UNGP responsibility
     attaches to the operator and the clause is flagged as non-derogating/remedy-stripping

   Keep the expected output mechanical: exact quins, exact shape violations, exact verdict
   status. That will make future ontology changes less hand-wavy.

4. Pin `values:State` before writing inverse-guard shapes.

   The plan says `NaturalPerson` and `LegalPerson` are siblings under `Agent`, while current
   `values.n3` models `State rdfs:subClassOf LegalPerson`. That may be defensible, but it
   must be explicit before category-error rules are compiled.

   Recommended options:

   - make `State` a direct sibling under `Agent`, with public-law personality represented as
     a role/capacity; or
   - keep `State` under `LegalPerson`, but add a discriminator such as
     `values:PublicLawPerson` vs `values:CommercialLegalPerson`, and key the inverse guard
     on that distinction.

5. Decide predicate packing for sense and temporal rules.

   `temporal_ltl.rs` compares full `NQuin.predicate` equality. If the compiler later packs
   property-path hashes into only part of the predicate word, temporal/sense formulas can
   compile successfully and then fail to match.

   Document one convention before wiring:

   - formulas compare the full packed predicate; or
   - formulas compare an extracted property-path portion, with a helper used everywhere.

## Platform-Agent Recommendations

1. Represent accountability as a state machine, not a final legal conclusion.

   The platform-agent section is strong, especially the refusal of "the algorithm did it"
   as a shield. In the ontology, avoid directly inferring final liability. Prefer states
   such as:

   - `values:ResponsibilityAlleged`
   - `values:ResponsibilityDerived`
   - `values:RequiresHumanReview`
   - `values:AdjudicatedResponsibility`
   - `values:SanctionableSubject`

   This keeps the runtime useful for routing, flags, and proofs without pretending that the
   engine itself is a court.

2. Separate responsibility, remedy, and sanction.

   Recommended terms:

   - `values:bearsResponsibility`
   - `values:owesRemedy`
   - `values:bearsSanction`
   - `values:sanctionKind`
   - `values:CivilSanction`
   - `values:CriminalSanction`
   - `values:CustodialSanction`
   - `values:MonetarySanction`

   A legal person can be the responsible operator and a civil/monetary sanction target.
   Custodial accountability should route toward accountable natural persons, but only as a
   review/adjudication path unless a jurisdiction-specific rule has been loaded.

3. Make jurisdiction anchoring explicit and queryable.

   `choiceOfLaw`, `operatesIn`, `affects`, and the user's region should be separate facts.
   The remedy-stripping flag should not erase the ToS fact; it should mark that the clause
   cannot derogate the human-rights baseline for the affected person in that context.

4. Add date-sensitive status fields for watchlist instruments.

   For not-yet-in-force or pending instruments, add fields such as:

   - `values:inForceStatus`
   - `values:notBeforeDate`
   - `values:lastChecked`
   - `values:watchlistStatus`

   This avoids freezing a 2026 planning note into durable ontology truth.

## Contractual-Incorporation Recommendations

1. Model formation as staged, not automatic.

   The plan already notes honest limits. Encode them as a state machine:

   - `values:Offer`
   - `values:Stipulation`
   - `values:Assent`
   - `values:RatifiedAgreement`
   - `values:BindingByContract`
   - `values:Rejected`
   - `values:Voidable`
   - `values:Voided`

   This preserves the key insight: user-held values credentials can become private-law
   terms, but only after the formation facts exist.

2. Reuse existing agreement machinery.

   The plan correctly points at RDF to `.q42`, CBOR-LD transport, deontic opcodes, and
   `SuspendedTransactionQueue`. The recommendation is to make that the required path for
   contract-grounded enforcement, rather than inventing a separate values-contract engine.

3. Tie capacity and duress into the same juridical-capacity role.

   `capacityToContract`, guardianship, coercion, and duress should live on the same
   time-qualified capacity model as "person before the law." That gives the plan one
   consistent way to handle minors, suspended capacity, coercive control, posthumous
   transforms, and guardian ratification.

## Tooling Recommendations

1. Make `validate_core_ontologies` native and strict.

   It should fail on:

   - Tier B files outside `mutable/`
   - missing `tier`, `legalForm`, `bindingStatus`, source, integrity hash, or review fields
   - scraper boilerplate in `values:originalText`
   - one-provision outliers in known multi-provision instruments
   - `NaturalPerson`-only rights reachable by `LegalPerson` or `ArtificialAgent` without
     a curated overlay
   - pending implementation markers that are accidentally treated as passing

2. Keep Python for acquisition tooling only.

   `build_index.py` can remain the offline dashboard generator. Runtime validation and
   shipped abuse-check reasoning should stay on the Rust/NQuin path.

3. Replace line-number claims in the plan with symbol names where possible.

   The updated plan cites useful line numbers, but line numbers drift quickly. Keep symbols
   like `execute_vm_frame`, `fire_registered_rules`, and `NativeDeonticEval` as the stable
   reference, with line numbers only as optional breadcrumbs.

4. Add source freshness to acquisition metadata.

   For current or pending legal instruments, acquisition records should carry:

   - source URL
   - retrieval date
   - source content hash
   - in-force/status checked date
   - generator version

   This matters especially for AI, business-and-human-rights, and mutable domestic-law
   material.

## Episteme Recommendation

The updated correction is reasonable: Episteme appears to be a prompt-layer framework in the
wider ecosystem, not a Rust crate in this repo. Treat any "Values/RightsGuardrail" Episteme
mode as an adapter that consumes compiled values/sense artifacts and returns proof-aware
prompts or traces. Do not let prompt-layer enforcement become a substitute for the native
MCP/portal/Sentinel checks.

## Revised Near-Term Work Packet

1. Add `tiering.n3` and update two representative instruments with tier/legal-form metadata.
2. Add `sense.n3` with the `person` worked example.
3. Extend `values.n3` with juridical capacity, platform-agent, accountability, guardian,
   contract-formation, and category-error terms.
4. Create overlay directories and one curated overlay fixture.
5. Add the five-case Section 4.3 trace as ontology fixtures plus a Rust smoke test through
   `fire_registered_rules` and/or `execute_vm_frame`.
6. Implement `validate_core_ontologies`.
7. Extend `build_index.py` into the governance gap report.
8. Re-segment the two OHCHR one-provision files and strip scraper boilerplate.
9. Resume acquisition only after the gate is green.

The plan is now strategically solid. The next risk is not conceptual; it is letting the plan
stay as prose instead of making the first executable, falsifiable spine.
