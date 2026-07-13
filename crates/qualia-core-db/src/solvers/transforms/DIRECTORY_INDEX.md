---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# transforms Index

## Functionality Overview
Comprehensive index of functionality for `transforms`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `fourier.rs`
  - `fn cadd`
  - `fn cmul`
  - `fn dft`
  - `fn dft_cpu`
  - `fn dft_accelerated`
  - `fn idft`
  - `fn re`
  - `fn dft_of_constant_is_an_impulse`
  - `fn dft_of_impulse_is_constant`
  - `fn accelerated_dft_matches_known_spectrum`
  - `fn accelerated_dft_agrees_with_cpu_reference`
  - `fn inverse_round_trips`
- 📄 `laplace.rs`
  - `enum LaplaceError`
  - `fn laplace_numeric`
  - `fn factorial`
  - `fn laplace_table`
  - `fn transform`
  - `fn is_var`
  - `fn eval_at_s`
  - `fn numeric_transforms_match_closed_forms`
  - `fn symbolic_table_powers_and_linearity`
  - `fn fails_closed_on_unrepresentable`
- 📄 `mod.rs`
- 📄 `ztransform.rs`
  - `fn cmul`
  - `fn cinv`
  - `fn csub`
  - `fn z_transform_finite`
  - `fn unit_step_z`
  - `fn geometric_z`
  - `fn finite_sequence_evaluates`
  - `fn closed_forms_match_truncated_sums`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
