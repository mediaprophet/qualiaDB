---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# fuzzy_query Index

## Functionality Overview
Comprehensive index of functionality for `fuzzy_query`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `evaluate.rs`
  - `fn annotate`
  - `fn collect_from`
  - `fn conjunctive_query`
  - `fn row`
  - `fn annotate_with_a_fuzzy_filter`
  - `fn collect_from_a_simulated_operator`
  - `fn collect_respects_cap`
  - `fn conjunctive_bgp_join_and_threshold`
  - `fn empty_patterns_is_empty`
- 📄 `membership.rs`
  - `fn clamp01`
  - `fn ramp_up`
  - `fn ramp_down`
  - `fn triangular`
  - `fn trapezoidal`
  - `fn approximately`
  - `fn much_greater_than`
  - `fn much_less_than`
  - `fn ramps_endpoints_and_interior`
  - `fn triangle_peaks_at_m`
  - `fn trapezoid_plateau`
  - `fn approximately_is_symmetric`
  - `fn comparators`
- 📄 `mod.rs`
  - `enum DegreeNorm`
  - `impl DegreeNorm`
  - `fn and`
  - `fn or`
  - `fn not`
  - `fn norm_dispatch_matches_modality_operators`
- 📄 `solution.rs`
  - `struct FuzzySolution`
  - `impl FuzzySolution`
  - `fn new`
  - `fn compatible`
  - `fn merge`
  - `struct FuzzyResultSet`
  - `impl FuzzyResultSet`
  - `fn from_solutions`
  - `fn push`
  - `fn len`
  - `fn is_empty`
  - `fn threshold`
  - `fn order_by_degree_desc`
  - `fn top_k`
  - `fn negate`
  - *(...and 10 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
