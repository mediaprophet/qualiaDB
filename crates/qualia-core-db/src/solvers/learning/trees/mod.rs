//! Tree-based methods (ISL ch 8) — CART trees and their ensembles.
//!
//! - [`decision_tree`] — CART regression (MSE) / classification (Gini) tree.
//!
//! Random forest (bagging + feature subsampling) and gradient boosting build on
//! the same tree and land next (build order in `stats_plan.md`).

pub mod boosting;
pub mod decision_tree;
pub mod random_forest;

pub use boosting::GradientBoosting;
pub use decision_tree::{Criterion, DecisionTree, TreeParams};
pub use random_forest::RandomForest;
