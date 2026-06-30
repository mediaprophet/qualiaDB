---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# hypothesis Index

## Functionality Overview
Comprehensive index of functionality for `hypothesis`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `anova.rs`
  - `struct AnovaResult`
  - `fn one_way_anova`
  - `fn detects_a_real_difference`
  - `fn no_difference_is_not_significant`
  - `fn matches_known_worked_example`
  - `fn guards_degenerate_input`
- 📄 `chi_square.rs`
  - `struct ChiSquareResult`
  - `fn chi_square_gof`
  - `fn chi_square_independence`
  - `fn gof_fair_die_is_not_rejected`
  - `fn gof_loaded_die_is_rejected`
  - `fn independence_known_example`
  - `fn independence_of_independent_table`
  - `fn guards_bad_shapes`
- 📄 `mod.rs`
- 📄 `nonparametric.rs`
  - `struct NonparametricResult`
  - `fn mcnemar`
  - `struct FriedmanResult`
  - `fn friedman`
  - `fn mcnemar_detects_disagreement`
  - `fn friedman_detects_a_consistent_ordering`
  - `fn friedman_no_difference_is_not_significant`
  - `fn guards`
- 📄 `t_tests.rs`
  - `struct TTest`
  - `fn one_sample_t`
  - `fn paired_t`
  - `struct TwoSampleTTest`
  - `fn two_sample_t`
  - `fn one_sample_real_p_value_not_a_threshold`
  - `fn one_sample_matches_known_statistic`
  - `fn paired_is_one_sample_of_differences`
  - `fn welch_vs_pooled_two_sample`
  - `fn identical_groups_are_not_significant`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
