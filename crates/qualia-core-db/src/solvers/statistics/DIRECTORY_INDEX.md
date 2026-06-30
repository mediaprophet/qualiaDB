---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# statistics Index

## Functionality Overview
Comprehensive index of functionality for `statistics`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Subdirectories
- 📁 `[distributions](distributions/DIRECTORY_INDEX.md)`
- 📁 `[hypothesis](hypothesis/DIRECTORY_INDEX.md)`

### Files & Exported Functionality
- 📄 `anomaly.rs`
  - `fn z_score_outliers`
  - `fn modified_z_score_outliers`
  - `fn tukey_fences`
  - `fn iqr_outliers`
  - `struct GrubbsResult`
  - `fn grubbs_test`
  - `fn mahalanobis_sq`
  - `fn is_multivariate_outlier`
  - `fn z_score_flags_the_obvious_spike`
  - `fn modified_z_is_robust_to_masking`
  - `fn tukey_fences_and_iqr_outliers`
  - `fn grubbs_detects_and_gates`
  - `fn mahalanobis_reduces_to_scaled_distance_for_identity`
  - `fn multivariate_gate_flags_a_far_point`
  - `fn fails_closed_on_degenerate`
- 📄 `correlation.rs`
  - `fn pearson`
  - `fn rank_into`
  - `fn kendall`
  - `fn spearman`
  - `fn correlation_p_value`
  - `fn pearson_perfect_and_anti`
  - `fn pearson_guards_and_zero_variance`
  - `fn rank_handles_ties`
  - `fn spearman_via_rank_then_pearson_is_monotonic_1`
  - `fn kendall_signs`
  - `fn spearman_is_one_for_monotone_nonlinear`
  - `fn correlation_p_value_significance`
- 📄 `descriptive.rs`
  - `fn sum`
  - `fn mean`
  - `fn variance`
  - `fn std_dev`
  - `fn median_sorted`
  - `fn median_in_place`
  - `fn min`
  - `fn max`
  - `fn argmax`
  - `fn covariance`
  - `fn central_moment`
  - `fn skewness`
  - `fn kurtosis`
  - `fn quantile_sorted`
  - `fn quantile_in_place`
  - *(...and 11 more)*
- 📄 `histogram.rs`
  - `struct HistRange`
  - `fn histogram_into`
  - `fn guards_empty`
  - `fn bins_uniform_data_and_conserves_count`
  - `fn degenerate_all_equal_goes_to_bin_zero`
  - `fn buffer_is_zeroed_first`
- 📄 `information.rs`
  - `fn entropy`
  - `fn entropy_from_counts`
  - `fn kl_divergence`
  - `fn cross_entropy`
  - `fn mutual_information_discrete`
  - `fn entropy_known_values`
  - `fn kl_is_zero_for_equal_and_positive_otherwise`
  - `fn mutual_information_detects_dependence`
  - `fn cross_entropy_decomposes`
  - `fn guards`
- 📄 `mod.rs`
- 📄 `regression.rs`
  - `struct LinearRegression`
  - `fn simple_linear_regression`
  - `fn exact_line_is_recovered`
  - `fn noisy_trend_matches_known_fit`
  - `fn no_relationship_is_not_significant`
  - `fn guards_degenerate_input`
- 📄 `robust.rs`
  - `fn trimmed_mean`
  - `fn winsorized_mean`
  - `fn median_abs_deviation`
  - `fn iqr`
  - `fn trimmed_mean_ignores_outliers`
  - `fn winsorized_mean_pulls_in_tails`
  - `fn mad_and_iqr_measure_robust_spread`
  - `fn robust_resists_a_single_contaminant`
  - `fn guards`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
