---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# regression Index

## Functionality Overview
Comprehensive index of functionality for `regression`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `bayesian.rs`
  - `struct BayesianLinear`
  - `fn design_row`
  - `impl BayesianLinear`
  - `fn fit`
  - `fn predict_row`
  - `fn predict`
  - `fn posterior_mean_approaches_ols_with_weak_prior`
  - `fn predictive_variance_grows_away_from_data`
  - `fn stronger_prior_shrinks_weights`
  - `fn guards`
- 📄 `lasso.rs`
  - `struct LassoModel`
  - `impl LassoModel`
  - `fn predict_row`
  - `fn predict`
  - `fn n_selected`
  - `fn soft_threshold`
  - `fn fit`
  - `fn lambda_zero_approaches_ols`
  - `fn large_penalty_zeros_all_coefficients`
  - `fn selects_the_relevant_predictor`
- 📄 `linear.rs`
  - `struct LinearModel`
  - `impl LinearModel`
  - `fn predict_row`
  - `fn predict`
  - `fn fit`
  - `fn recovers_exact_plane`
  - `fn matches_simple_regression_for_one_predictor`
  - `fn detects_collinear_predictors`
  - `fn guards_insufficient_data`
  - `fn significant_predictor_has_small_p`
- 📄 `mod.rs`
- 📄 `pcr.rs`
  - `struct PcrModel`
  - `impl PcrModel`
  - `fn fit`
  - `fn predict_row`
  - `fn predict`
  - `fn n_components`
  - `fn full_components_matches_ols_fit_quality`
  - `fn one_component_captures_dominant_direction`
  - `fn guards`
- 📄 `pls.rs`
  - `struct PlsModel`
  - `impl PlsModel`
  - `fn predict_row`
  - `fn predict`
  - `fn fit`
  - `fn finalize`
  - `fn full_components_matches_ols`
  - `fn one_component_tracks_covariance_direction`
  - `fn guards`
- 📄 `ridge.rs`
  - `struct RidgeModel`
  - `impl RidgeModel`
  - `fn predict_row`
  - `fn predict`
  - `fn fit`
  - `fn lambda_zero_matches_ols`
  - `fn penalty_shrinks_coefficients`
  - `fn stable_on_collinear_where_ols_fails`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
