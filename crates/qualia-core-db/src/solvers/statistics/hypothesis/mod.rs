//! Hypothesis tests — real p-values from the [`distributions`](super::distributions)
//! library, zero ad-hoc thresholds.
//!
//! This replaces the prior `one_sample_t`, whose "p-value" was a placeholder
//! (`|t| > 1.96 ⇒ 0.05 else 0.1`) and whose CI used the fixed 1.96 normal critical
//! value regardless of sample size. Every test here draws its tail probability from
//! the exact Student-t / F / χ² CDF, and t-CIs use the t critical value for the
//! actual degrees of freedom.
//!
//! Submodules (PROJECT RULE §11): [`t_tests`] (one-sample / paired / two-sample),
//! [`anova`] (one-way F-test), [`chi_square`] (goodness-of-fit / independence).
//! Specialized libraries map these results onto their own domain result types.

pub mod anova;
pub mod chi_square;
pub mod nonparametric;
pub mod t_tests;

pub use anova::{one_way_anova, AnovaResult};
pub use chi_square::{chi_square_gof, chi_square_independence, ChiSquareResult};
pub use nonparametric::{friedman, mcnemar, FriedmanResult, NonparametricResult};
pub use t_tests::{one_sample_t, paired_t, two_sample_t, TTest, TwoSampleTTest};
