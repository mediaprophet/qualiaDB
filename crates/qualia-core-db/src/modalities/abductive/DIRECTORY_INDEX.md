---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# abductive Index

## Functionality Overview
Comprehensive index of functionality for `abductive`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `atms.rs`
  - `fn env_subset`
  - `fn is_nogood`
  - `fn label_add`
  - `fn label_holds`
  - `fn holds_in`
  - `fn label_maintains_minimal_environments`
  - `fn nogoods_are_superset_closed`
  - `fn belief_requires_a_consistent_supporting_context`
- 📄 `mod.rs`
  - `fn abductive_explanation`
  - `fn minimal_explanation`
  - `fn counter_abduction`
  - `fn edge`
  - `fn finds_root_explanation`
  - `fn minimal_explanation_collapses_shared_roots`
  - `fn counter_abduction_prunes_refuted`
- 📄 `probabilistic.rs`
  - `struct Hypothesis`
  - `fn bayesian_posteriors`
  - `fn best_hypothesis`
  - `fn close`
  - `fn posteriors_normalise_and_rank`
  - `fn no_mass_yields_none_and_zero_evidence`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
