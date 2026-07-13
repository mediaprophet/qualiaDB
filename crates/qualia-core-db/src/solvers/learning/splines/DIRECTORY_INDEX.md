---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# splines Index

## Functionality Overview
Comprehensive index of functionality for `splines`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `gam.rs`
  - `struct Gam`
  - `impl Gam`
  - `fn fit`
  - `fn predict_row`
  - `fn predict`
  - `fn fits_an_additive_nonlinear_surface`
  - `fn recovers_a_linear_additive_model`
  - `fn guards`
- 📄 `mod.rs`
  - `struct RegressionSpline`
  - `impl RegressionSpline`
  - `fn fit`
  - `fn predict_one`
  - `fn predict`
  - `fn polynomial_regression`
  - `fn polynomial_recovers_a_quadratic_exactly`
  - `fn cubic_spline_fits_a_kinked_curve`
  - `fn guards`
- 📄 `smoothing.rs`
  - `struct SmoothingSpline`
  - `impl SmoothingSpline`
  - `fn fit`
  - `fn predict_one`
  - `fn predict`
  - `fn roughness`
  - `fn lambda_zero_matches_regression_spline`
  - `fn larger_lambda_is_smoother`
  - `fn guards`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
