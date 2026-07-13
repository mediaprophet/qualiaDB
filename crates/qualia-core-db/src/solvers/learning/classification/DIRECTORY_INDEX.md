---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# classification Index

## Functionality Overview
Comprehensive index of functionality for `classification`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `discriminant.rs`
  - `struct ClassStats`
  - `fn class_stats`
  - `struct LdaModel`
  - `impl LdaModel`
  - `fn fit`
  - `fn predict_row`
  - `fn predict`
  - `struct QdaModel`
  - `impl QdaModel`
  - `fn cholesky_of`
  - `fn data`
  - `fn lda_separates_classes`
  - `fn qda_separates_classes`
  - `fn qda_fails_closed_on_too_few_per_class`
- 📄 `knn.rs`
  - `struct KnnClassifier`
  - `impl KnnClassifier`
  - `fn fit`
  - `fn predict_row`
  - `fn predict`
  - `fn sq_dist`
  - `fn classifies_by_nearest_neighbours`
  - `fn k_one_is_the_single_nearest`
  - `fn guards`
- 📄 `mod.rs`
- 📄 `naive_bayes.rs`
  - `struct GaussianNb`
  - `impl GaussianNb`
  - `fn fit`
  - `fn log_score`
  - `fn predict_row`
  - `fn predict`
  - `fn separates_two_gaussian_classes`
  - `fn respects_priors`
  - `fn guards`
- 📄 `svm.rs`
  - `enum Kernel`
  - `impl Kernel`
  - `fn eval`
  - `struct Svm`
  - `struct Lcg`
  - `impl Lcg`
  - `fn below`
  - `fn fill_kernel_cpu`
  - `fn fit`
  - `impl Svm`
  - `fn decision_row`
  - `fn predict_row`
  - `fn n_support_vectors`
  - `fn linear_svm_separates_linearly_separable`
  - `fn rbf_svm_handles_nonlinear_boundary`
  - *(...and 1 more)*
- 📄 `svm_multiclass.rs`
  - `struct MulticlassSvm`
  - `impl MulticlassSvm`
  - `fn fit_one_vs_rest`
  - `fn predict_row`
  - `fn predict`
  - `fn separates_three_classes`
  - `fn guards`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
