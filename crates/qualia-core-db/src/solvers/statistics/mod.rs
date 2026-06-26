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

pub mod correlation;
pub mod descriptive;
pub mod histogram;
pub mod hypothesis;

pub use correlation::{kendall, pearson, rank_into};
pub use descriptive::{
    max, mean, median_in_place, median_sorted, min, std_dev, sum, variance,
};
pub use histogram::{histogram_into, HistRange};
pub use hypothesis::{one_sample_t, TTest};
