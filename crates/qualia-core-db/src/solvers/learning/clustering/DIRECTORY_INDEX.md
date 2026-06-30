---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# clustering Index

## Functionality Overview
Comprehensive index of functionality for `clustering`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `gmm.rs`
  - `struct GmmModel`
  - `fn log_gauss_diag`
  - `fn log_sum_exp`
  - `fn fit`
  - `fn two_blobs`
  - `fn recovers_two_components`
  - `fn log_likelihood_is_finite_and_weights_normalised`
  - `fn guards`
- 📄 `hierarchical.rs`
  - `enum Linkage`
  - `struct Hierarchical`
  - `fn sq_dist`
  - `fn cluster_distance`
  - `impl Hierarchical`
  - `fn fit`
  - `fn labels`
  - `fn find`
  - `fn n_merges`
  - `fn separates_two_obvious_groups`
  - `fn k_equals_n_is_all_singletons_and_k_one_is_all_together`
  - `fn complete_and_single_linkage_both_run`
  - `fn guards`
- 📄 `kmeans.rs`
  - `struct KMeansModel`
  - `impl KMeansModel`
  - `fn predict_row`
  - `struct Lcg`
  - `impl Lcg`
  - `fn unit`
  - `fn sq_dist`
  - `fn nearest`
  - `fn kmeans_pp`
  - `fn fit`
  - `fn recovers_three_separated_blobs`
  - `fn single_cluster_centroid_is_the_mean`
  - `fn guards`
- 📄 `mod.rs`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
