---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# metrics Index

## Functionality Overview
Comprehensive index of functionality for `metrics`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `classification.rs`
  - `fn accuracy`
  - `struct ConfusionBinary`
  - `impl ConfusionBinary`
  - `fn total`
  - `fn precision`
  - `fn recall`
  - `fn specificity`
  - `fn f1`
  - `fn confusion_binary`
  - `fn roc_auc`
  - `fn log_loss`
  - `fn accuracy_basic`
  - `fn confusion_rates`
  - `fn auc_perfect_and_random`
  - `fn auc_known_value`
  - *(...and 1 more)*
- 📄 `mod.rs`
- 📄 `regression.rs`
  - `fn mse`
  - `fn rmse`
  - `fn mae`
  - `fn r2_score`
  - `fn perfect_prediction`
  - `fn known_values`
  - `fn mean_predictor_is_zero_r2`
  - `fn guards`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
