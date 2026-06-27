//! Statistics solver — the single, canonical home for numeric statistics.
//!
//! Zero-allocation kernels over caller-owned slices, a sibling to
//! `solvers::linear_algebra`. This is where descriptive statistics, hypothesis
//! tests, correlation, and binning live for the *whole* engine.
//!
//! **Composition rule (Modality-First):** specialized/domain libraries
//! (`specialized_libs::statistical_computing`, `machine_learning`, …) marshal
//! their domain data into a slice and call these functions. They MUST NOT carry
//! their own `mean`/`variance`/`correlation` re-implementations. See
//! `MODALITY_FIRST_CONSOLIDATION.md`.
//!
//! Probabilistic *logic* (Bayesian networks, truth-degree reasoning over quins)
//! is a separate concern and lives in `modalities::probabilistic`; it may call
//! into here for numeric work, but the two are not merged.

pub mod anomaly;
pub mod correlation;
pub mod descriptive;
pub mod distributions;
pub mod histogram;
pub mod hypothesis;
pub mod information;
pub mod regression;
pub mod robust;

pub use correlation::{correlation_p_value, kendall, pearson, rank_into, spearman};
pub use descriptive::{
    covariance, kurtosis, max, mean, median_in_place, median_sorted, min, quantile_in_place,
    quantile_sorted, skewness, std_dev, sum, variance,
};
pub use histogram::{histogram_into, HistRange};
pub use hypothesis::{
    chi_square_gof, chi_square_independence, one_sample_t, one_way_anova, paired_t, two_sample_t,
    AnovaResult, ChiSquareResult, TTest, TwoSampleTTest,
};
pub use regression::{simple_linear_regression, LinearRegression};
pub use information::{cross_entropy, entropy, kl_divergence, mutual_information_discrete};
pub use robust::{iqr, median_abs_deviation, trimmed_mean, winsorized_mean};
