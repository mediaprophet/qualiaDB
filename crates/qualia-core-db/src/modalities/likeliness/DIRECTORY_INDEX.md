---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# likeliness Index

## Functionality Overview
Comprehensive index of functionality for `likeliness`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `algebra.rs`
  - `fn not`
  - `fn and`
  - `fn or`
  - `fn combine_premises`
  - `fn combine_routes`
  - `fn negation_is_an_involution_reflecting_the_scale`
  - `fn meet_and_join_are_min_and_max`
  - `fn de_morgan_laws_hold`
  - `fn kleene_no_excluded_middle_no_contradiction`
  - `fn vacuous_folds_are_top_and_bottom`
- 📄 `inference.rs`
  - `fn attenuate`
  - `fn modus_ponens`
  - `fn infer_chain`
  - `fn rebut`
  - `fn revise`
  - `fn modus_ponens_is_weakest_link`
  - `fn chains_attenuate_with_length`
  - `fn rebuttal_defeats_in_proportion_to_strength`
  - `fn revision_combines_support_and_rebuttal`
  - `fn attenuate_saturates`
- 📄 `mod.rs`
  - `enum Likeliness`
  - `impl Likeliness`
  - `fn from_level`
  - `fn level_round_trips_and_orders`
  - `fn from_level_saturates`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
