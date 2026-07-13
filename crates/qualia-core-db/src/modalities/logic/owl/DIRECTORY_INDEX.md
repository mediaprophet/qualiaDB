---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# owl Index

## Functionality Overview
Comprehensive index of functionality for `owl`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `materialize.rs`
  - `struct RdfTriple`
  - `impl RdfTriple`
  - `struct ChainAxiom`
  - `struct DisjointnessViolation`
  - `enum MaterializeError`
  - `struct MaterializeSummary`
  - `fn contains`
  - `fn try_push`
  - `fn materialize_owl_rl`
  - `fn h`
  - `fn subclass_transitivity_and_type_propagation`
  - `fn domain_and_range_typing`
  - `fn transitive_and_inverse_properties`
  - `fn property_chain_unrolling`
  - `fn disjointness_isolation_does_not_halt`
  - *(...and 2 more)*
- 📄 `mod.rs`
- 📄 `shacl_convert.rs`
  - `struct OwlClass`
  - `struct OwlProperty`
  - `struct HealthcareOwlModel`
  - `struct RadlexRelation`
  - `enum OwlToShaclError`
  - `impl std`
  - `fn fmt`
  - `fn is_blank_or_anon`
  - `fn local_name`
  - `fn shape_name_for_uri`
  - `fn curie_for_uri`
  - `fn xsd_from_range`
  - `fn ingest_owl_triple`
  - `fn parse_healthcare_owl_turtle`
  - `fn parse_healthcare_owl_n3`
  - *(...and 18 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
