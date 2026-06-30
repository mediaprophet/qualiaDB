---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# distributions Index

## Functionality Overview
Comprehensive index of functionality for `distributions`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `chi_squared.rs`
  - `fn pdf`
  - `fn cdf`
  - `fn upper_p`
  - `fn quantile`
  - `fn cdf_is_exponential_for_k_two`
  - `fn known_critical_values`
  - `fn quantile_inverts_cdf`
- 📄 `fisher_f.rs`
  - `fn pdf`
  - `fn cdf`
  - `fn upper_p`
  - `fn quantile`
  - `fn known_critical_values`
  - `fn cdf_monotone_and_bounds`
  - `fn quantile_inverts_cdf`
- 📄 `mod.rs`
  - `fn invert_cdf_recovers_a_linear_cdf`
- 📄 `multivariate_normal.rs`
  - `fn log_pdf`
  - `fn pdf`
  - `struct Rng`
  - `impl Rng`
  - `fn unit`
  - `fn gaussian`
  - `fn sample`
  - `fn mle`
  - `fn reduces_to_univariate_normal`
  - `fn independent_factorises`
  - `fn log_pdf_peaks_at_the_mean`
  - `fn mle_recovers_planted_parameters`
  - `fn singular_covariance_is_none`
- 📄 `normal.rs`
  - `fn standard_pdf`
  - `fn standard_cdf`
  - `fn pdf`
  - `fn cdf`
  - `fn standard_quantile`
  - `fn quantile`
  - `fn two_sided_p`
  - `fn cdf_known_quantiles`
  - `fn pdf_peak_and_symmetry`
  - `fn quantile_inverts_cdf`
  - `fn general_params_shift_and_scale`
  - `fn two_sided_p_value`
- 📄 `special.rs`
  - `fn ln_gamma`
  - `fn gamma`
  - `fn gammp`
  - `fn gammq`
  - `fn gser`
  - `fn gcf`
  - `fn betai`
  - `fn betacf`
  - `fn erf`
  - `fn erfc`
  - `fn ln_gamma_known_values`
  - `fn erf_known_values`
  - `fn gammp_is_exponential_cdf_for_a_one`
  - `fn gammp_gammq_complementary`
  - `fn betai_endpoints_and_symmetry`
- 📄 `students_t.rs`
  - `fn pdf`
  - `fn cdf`
  - `fn two_sided_p`
  - `fn upper_p`
  - `fn quantile`
  - `fn cdf_symmetry_and_center`
  - `fn known_critical_values`
  - `fn two_sided_p_matches_tail`
  - `fn quantile_inverts_cdf`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
