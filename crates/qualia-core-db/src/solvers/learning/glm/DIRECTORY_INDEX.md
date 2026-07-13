---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# glm Index

## Functionality Overview
Comprehensive index of functionality for `glm`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `family.rs`
  - `enum Family`
  - `impl Family`
  - `fn inv_link`
  - `fn dmu_deta`
  - `fn variance`
  - `fn unit_deviance`
  - `fn start_mu`
  - `fn logistic_link_round_trip`
  - `fn poisson_link`
  - `fn deviance_is_zero_at_perfect_fit`
- 📄 `mod.rs`
  - `struct GlmModel`
  - `impl GlmModel`
  - `fn eta_row`
  - `fn predict_row`
  - `fn predict`
  - `fn fit`
  - `fn fit_logistic`
  - `fn fit_poisson`
  - `fn logistic_recovers_positive_trend`
  - `fn logistic_significance_and_inference`
  - `fn poisson_recovers_log_linear_rate`
  - `fn guards`
- 📄 `multinomial.rs`
  - `struct MultinomialLogistic`
  - `fn design_row`
  - `fn softmax`
  - `impl MultinomialLogistic`
  - `fn fit`
  - `fn predict_proba_row`
  - `fn predict_row`
  - `fn classifies_three_separated_clusters`
  - `fn guards`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
