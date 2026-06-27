//! Dimensionality reduction (ISL ch 12, PRML ch 12). Built on `linear_algebra`
//! (eigen/SVD). Feeds the engine's 10D→5D NQuin relevance router.
//!
//! - [`pca`] — Principal Component Analysis (covariance eigendecomposition).
//!
//! Probabilistic PCA / kernel PCA and PCR/PLS regression land here next
//! (build order in `stats_plan.md`).

pub mod pca;
pub mod som;

pub use pca::{fit as fit_pca, Pca};
pub use som::Som;
