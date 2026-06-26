//! Classification (ISL ch 4) — generative and instance-based classifiers.
//!
//! - [`knn`] — k-nearest-neighbours (lazy, `AllPairs`).
//! - [`naive_bayes`] — Gaussian naive Bayes (generative, `Reduction`).
//!
//! LDA / QDA (shared / per-class Gaussian discriminants) land here next, on the
//! `linear_algebra` covariance solve (build order in `stats_plan.md`).

pub mod discriminant;
pub mod knn;
pub mod naive_bayes;

pub use discriminant::{LdaModel, QdaModel};
pub use knn::KnnClassifier;
pub use naive_bayes::GaussianNb;
