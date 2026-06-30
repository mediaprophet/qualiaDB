---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# trees Index

## Functionality Overview
Comprehensive index of functionality for `trees`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `bart.rs`
  - `struct Node`
  - `struct Tree`
  - `impl Tree`
  - `fn root`
  - `fn leaf_of`
  - `fn predict`
  - `fn leaves`
  - `fn nog_nodes`
  - `struct Rng`
  - `impl Rng`
  - `fn unit`
  - `fn below`
  - `fn gaussian`
  - `fn gamma`
  - `struct Bart`
  - *(...and 13 more)*
- 📄 `boosting.rs`
  - `struct GradientBoosting`
  - `impl GradientBoosting`
  - `fn fit_regressor`
  - `fn predict_row`
  - `fn predict`
  - `fn n_estimators`
  - `fn boosting_reduces_error_with_more_stages`
  - `fn single_stage_is_init_plus_one_tree`
  - `fn guards`
- 📄 `decision_tree.rs`
  - `enum Criterion`
  - `struct TreeParams`
  - `impl Default`
  - `fn default`
  - `struct Node`
  - `struct DecisionTree`
  - `struct Lcg`
  - `impl Lcg`
  - `fn next`
  - `fn impurity`
  - `fn leaf_value`
  - `struct Builder`
  - `fn build`
  - `fn candidate_features`
  - `fn push_leaf`
  - *(...and 12 more)*
- 📄 `mod.rs`
- 📄 `random_forest.rs`
  - `struct RandomForest`
  - `struct Lcg`
  - `impl Lcg`
  - `fn below`
  - `fn default_features_classification`
  - `fn default_features_regression`
  - `fn fit_inner`
  - `impl RandomForest`
  - `fn fit_regressor`
  - `fn fit_classifier`
  - `fn predict_row`
  - `fn predict_class`
  - `fn predict`
  - `fn n_trees`
  - `fn regression_forest_fits_a_nonlinear_trend`
  - *(...and 2 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
