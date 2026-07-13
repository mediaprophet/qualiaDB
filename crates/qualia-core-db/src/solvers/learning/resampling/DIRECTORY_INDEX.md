---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# resampling Index

## Functionality Overview
Comprehensive index of functionality for `resampling`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `bootstrap.rs`
  - `struct Lcg`
  - `impl Lcg`
  - `fn next_below`
  - `fn bootstrap_indices`
  - `struct BootstrapResult`
  - `fn bootstrap_estimate`
  - `enum CiMethod`
  - `struct BootstrapCi`
  - `fn bootstrap_ci`
  - `fn resample_indices_in_range_and_reproducible`
  - `fn bootstrap_se_of_mean_matches_clt`
  - `fn percentile_ci_brackets_the_true_mean`
  - `fn bca_runs_and_is_a_valid_interval`
  - `fn ci_works_for_a_nonlinear_statistic`
  - `fn guards`
- 📄 `folds.rs`
  - `struct Lcg`
  - `impl Lcg`
  - `fn next_below`
  - `fn shuffle`
  - `struct Fold`
  - `fn k_fold`
  - `fn loocv`
  - `fn train_test_split`
  - `fn k_fold_partitions_every_row_once_as_test`
  - `fn k_fold_sizes_balanced`
  - `fn loocv_has_n_folds_of_one`
  - `fn train_test_split_sizes_and_disjoint`
  - `fn guards`
- 📄 `mod.rs`
  - `fn gather_rows`
  - `fn cross_val_score`
  - `fn cross_validates_a_linear_model`
  - `fn loocv_runs_n_folds`
- 📄 `permutation.rs`
  - `struct PermutationResult`
  - `struct Lcg`
  - `impl Lcg`
  - `fn below`
  - `fn two_sample_test`
  - `fn detects_a_real_difference`
  - `fn no_difference_is_not_significant`
  - `fn works_for_a_difference_of_medians`
  - `fn guards`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
