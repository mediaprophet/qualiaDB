//! Statistical learning (ISL) — predictive estimators built **on** the engine's
//! existing foundation, never duplicating it (see `stats_plan.md`).
//!
//! Reuses: `solvers::linear_algebra` (gemm/qr/cholesky/eigen/svd) for the linear
//! algebra, `solvers::statistics` (descriptive/distributions/correlation) for moments
//! and p-values, and `platform::compute_bridge` for per-kernel-class dispatch.
//!
//! Categories (one method-family per sub-library, PROJECT RULE §13):
//! [`metrics`], [`preprocessing`], [`regression`], [`glm`], [`classification`],
//! [`resampling`], [`dimensionality`], [`clustering`], [`trees`], [`splines`],
//! [`survival`], [`multiple_testing`].

// Declared as each method-family lands (build order in `stats_plan.md`).
pub mod classification;
pub mod clustering;
pub mod dimensionality;
pub mod gaussian_process;
pub mod glm;
pub mod metrics;
pub mod multiple_testing;
pub mod preprocessing;
pub mod regression;
pub mod resampling;
pub mod sampling;
pub mod sequential;
pub mod splines;
pub mod survival;
pub mod trees;

/// Errors common to the learning estimators. Estimators **fail closed** (return an
/// error) rather than emit a fabricated fit — consistent with the engine-wide
/// honesty rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LearningError {
    /// Input shapes are inconsistent (lengths / row×col mismatch).
    InvalidDimension,
    /// Not enough data for the requested fit (e.g. fewer samples than parameters).
    InsufficientData,
    /// The system is singular / rank-deficient (e.g. collinear predictors) — fail
    /// closed instead of returning a meaningless solution.
    Singular,
    /// An iterative fit did not converge within its iteration budget.
    NotConverged,
}

impl core::fmt::Display for LearningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LearningError::InvalidDimension => write!(f, "inconsistent input dimensions"),
            LearningError::InsufficientData => write!(f, "insufficient data for the requested fit"),
            LearningError::Singular => write!(f, "singular / rank-deficient system (e.g. collinear predictors)"),
            LearningError::NotConverged => write!(f, "iterative fit did not converge"),
        }
    }
}
impl std::error::Error for LearningError {}

impl From<crate::solvers::SolversError> for LearningError {
    fn from(e: crate::solvers::SolversError) -> Self {
        use crate::solvers::SolversError as E;
        match e {
            E::InvalidDimension => LearningError::InvalidDimension,
            E::SingularMatrix => LearningError::Singular,
            _ => LearningError::InvalidDimension,
        }
    }
}
