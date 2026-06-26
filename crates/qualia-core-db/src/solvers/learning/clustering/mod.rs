//! Clustering & mixture models (ISL ch 12, PRML ch 9).
//!
//! - [`kmeans`] — Lloyd's algorithm with k-means++ seeding.
//! - [`gmm`] — diagonal-covariance Gaussian mixture via EM (seeded by k-means).
//!
//! Hierarchical/agglomerative clustering lands here next (build order in
//! `stats_plan.md`).

pub mod gmm;
pub mod kmeans;

pub use gmm::{fit as fit_gmm, GmmModel};
pub use kmeans::{fit as fit_kmeans, KMeansModel};
