# ADR 0002 — RDF 1.2 triple terms and reifiers; legacy RDF-star as compatibility only

**Status:** Accepted (Editor's Draft 0.1, provisional — not a W3C/OGC standard)
**Date:** 2026-07-13
**Requirements:** QISP-R02, QISP-R01
**Plan sections:** §0 layer 1, §2.2 point 2, §1.1 (RDF-star row), §5.3, Phase 1

## Context

The codebase has legacy RDF-star syntax and parsers, and hashed embedded triples
(`Pattern::StarTriple`). The source conversation used the older annotation form
`<< ?s ?p ?o >> :authorizedBy ?vc`. RDF 1.2 changes this: metadata about a triple is expressed with
a **triple term** `<<( ?s ?p ?o )>>` plus a **reifier**, normally via `rdf:reifies`. QISP query 03
(§5.3) depends on the new model. RDF 1.2 / SPARQL 1.2 are still moving through the W3C process, so
the exact dated snapshot must be pinned (decision QISP-D03) and draft support must be distinguished
from stable SPARQL 1.1 conformance.

## Decision

Adopt the **RDF 1.2 triple-term + reifier model** as the QISP semantic layer for statement-level
metadata (authorization, provenance, validity). Concretely:

- represent triple terms as `<<( s p o )>>` and attach metadata through a reifier resource with
  `rdf:reifies` (as in §5.3), not the legacy annotation syntax;
- add `VERSION "1.2"` announcement parsing;
- **retain a documented legacy RDF-star parser mode** for input compatibility while migrating — the
  older `<< >> :p ?o` form is accepted only in that explicit mode and is either translated or
  rejected with a precise mode error (never silently reinterpreted);
- pin the exact dated RDF 1.2 Concepts / concrete-syntax and SPARQL 1.2 snapshots plus the
  `rdf-tests` commit used, and advertise this as the `qisp:CoreRdf12` / `qisp:Sparql12Query` draft
  conformance classes — separate from stable SPARQL 1.1 discovery.

## Consequences

- **Positive:** statement-level authorization/provenance is expressed in the upstream-aligned way,
  round-trips through RDF 1.2 syntax (QISP-R02 test target), and does not fork the general grammar
  (the full-SPARQL plan owns the grammar; QISP extends once, §10.2). Ordinary `/sparql` stays
  conformant (QISP-R01).
- **Negative / cost:** RDF 1.2 is a moving target — pinned snapshots must be updated deliberately,
  with migration notes; two syntaxes (new + legacy mode) must be maintained during the transition;
  serialization must round-trip both.
- **Risk:** if upstream RDF 1.2 semantics shift before Recommendation, terms may need re-pinning;
  the 0.1 maturity label explicitly permits this and forbids claiming stability.
