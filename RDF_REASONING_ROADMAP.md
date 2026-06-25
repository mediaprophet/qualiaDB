# RDF Reasoning Modalities — Roadmap & Completeness Tracker

**Purpose.** Timothy directed (2026-06-25): get the RDF-related logic modalities **all done completely** —
alongside the production-excellence audit. This is the tracked enumeration + status for that net-new
workstream (the audit completeness lives in `.dev-docs/to-do/audit_production_excellence_tasks.md`; boundaries
in `AUDIT_BOUNDARY_DEFERRALS.md`). Same bar: an independent reviewer calls each **complete**, real tested code,
zero-heap where it's a hot path, honest flags only for genuine boundaries.

Branch `0.0.20-production-excellence`. Living doc — checked off with commit ids as built.

---

## Status legend
`[x]` complete · `[~]` partial/boundary (with reason) · `[ ]` not started · `(vocab)` = vocabulary present, no reasoner

## Already supported (verified in code)
RDFS subsumption (`dl::check_subsumption_quin`), OWL (RL materialization in progress, `owl.rs`), SHACL
(validation + firewall), SPARQL (query layer), N3 **parsing** (`n3_parser.rs`, separate worktree), SKOS
(closeMatch/exactMatch gate), PROV-O (provenance/WAL), ODRL (deontic OP_FORBID), VC-DM, DID.
CogAI = **vocabulary only** (`cogai/ont#believes|queries|observes` in `epistemic.rs`).

---

## A. Rule / inference-interchange layer (the biggest gap)

- [ ] **RDF-star (RDF 1.2) — quoted triples / statements about statements.** *Most mission-aligned.* Native
  substrate for allegation↔adjudication (`responsibility.rs`), provenance attestation, trust claims *about* a
  triple, and the Curation Directive ("machine proposes a triple, human attests it"). Deepens the
  out-of-band-remainder invariant. (Currently approximated by the allegation model.)
- [ ] **Full N3Logic** — parser exists; the substance is the **built-in libraries** (`math:`, `string:`,
  `list:`, `log:implies`, `time:`, `crypto:`), formulae + quantification, forward/backward chaining (cwm/EYE
  semantics). **Zero built-ins in code today.** The human-centric web's native rule language (Timothy flagged it).
- [ ] **Datalog / RDFS+OWL-RL entailment ruleset** — stratified Datalog (± negation) + the RDFS entailment
  rules (rdfs1–13) + OWL-RL ruleset. `asp.rs` is a superset; `fire_guard_rules` forward-chains; no explicit
  RDFS/RL materialization engine.
- [ ] **ShEx (Shape Expressions)** — **ADOPTED (ADR 0009) but UNBUILT.** Per the ADR: ShEx *describes*, SHACL
  *enforces* (one source, no drift). Scope: **recursion** (guardianship/delegation/lineage DAGs — SHACL recursion
  is weak), the compact **ShExC** contract syntax, and **Wikidata EntitySchema** interop. Needs: ShExC parse,
  triple-expression eval (EachOf/OneOf/ShapeAnd/ShapeOr), recursive shape conformance. See [[project-shex-vs-shacl-decision]].
- [ ] **W3C CogAI — chunks & rules.** Vocab present; the **production system** (chunk graph + rule-firing
  cognitive cycle, plausible reasoning) is not. A production-rule paradigm (condition→action).
- [ ] **SWRL** — OWL + Horn rules (the Protégé-ubiquitous OWL↔rules bridge).
- [ ] **RIF** — Rule Interchange Format: RIF-Core, RIF-BLD (Horn), RIF-PRD (production rules; overlaps CogAI).

## B. OWL profiles / DL
- [ ] **OWL 2 EL** — SNOMED-scale biomedical ontologies (relevant to the medical/rights ontologies).
- [ ] **OWL 2 QL / DL-Lite** — query rewriting / ontology-based data access over the graph.
- [~] **OWL 2 RL** — materialization in progress (`owl.rs`, audit item).
- [~] **Full SROIQ tableau** — structural constructs done; the model-construction tableau is a boundary
  (`AUDIT_BOUNDARY_DEFERRALS.md`, `dl.rs`).

## C. Domain-standard reasoning vocabularies
- [ ] **GeoSPARQL** — defines **RCC-8** (the `spatio_temporal` flag). The *qualitative* relations
  (within/contains/touches over the jurisdiction hierarchy) ARE NQuin-encodable — `deontic_compose::obligation_applies_in`
  already does `within`. Only *exact float-geometry* RCC-8 hits the 48-byte invariant. → support qualitatively.
- [ ] **OWL-Time** — the RDF temporal-entity vocabulary; the standards face of `interval_reasoning` (Allen) +
  `temporal_ltl` already built.

## D. SHACL advanced
- [ ] **SHACL-AF (Advanced Features)** — SHACL *rules* (triple/SPARQL rules) + functions: turns SHACL from
  validate-only into an **inference** language.
- [ ] **SPARQL entailment regimes** — querying under RDFS/OWL/RIF entailment.

---

## Recommended order (mission-first)
RDF-star → full N3Logic built-ins → Datalog/RDFS-RL → **ShEx** (recursion + ShExC) → CogAI production rules →
GeoSPARQL-qualitative + OWL-Time → SWRL/RIF/EL/QL/SHACL-AF (interop tail).

**Sequencing (Timothy: "get them all done completely"):** finish the audit (≈39 items, nearest done), folding in
the overlaps (GeoSPARQL↔`spatio_temporal`, ShEx↔`logic/`), then build the standalone RDF modalities top-to-bottom.
