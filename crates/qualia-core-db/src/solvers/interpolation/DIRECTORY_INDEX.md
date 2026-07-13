---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# interpolation Index

## Functionality Overview
Comprehensive index of functionality for `interpolation`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `lagrange.rs`
  - `fn validate`
  - `fn lagrange_eval`
  - `fn newton_coefficients`
  - `fn newton_eval`
  - `fn interpolant_passes_through_nodes`
  - `fn reproduces_a_quadratic_exactly`
  - `fn newton_matches_lagrange`
  - `fn fails_closed`
- 📄 `least_squares.rs`
  - `fn poly_fit`
  - `fn poly_eval`
  - `fn gauss_solve`
  - `fn recovers_a_line_from_collinear_points`
  - `fn recovers_a_parabola_exactly`
  - `fn least_squares_minimises_on_noisy_data`
  - `fn fails_closed`
- 📄 `mod.rs`
  - `enum InterpolationError`
  - `impl core`
  - `fn fmt`
  - `impl std`
- 📄 `spline.rs`
  - `fn linear_interp`
  - `struct CubicSpline`
  - `impl CubicSpline`
  - `fn natural`
  - `fn eval`
  - `fn thomas`
  - `fn spline_passes_through_nodes`
  - `fn natural_spline_reproduces_a_line`
  - `fn linear_interpolation_midpoints`
  - `fn fails_closed`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
