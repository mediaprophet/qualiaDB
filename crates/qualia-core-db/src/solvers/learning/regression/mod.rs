//! Regression estimators (ISL ch 3, 6). Built on `linear_algebra` + `statistics`.
//!
//! - [`linear`] — multiple OLS with full inference.
//!
//! Ridge / lasso / PCR / PLS land here next (build order in `stats_plan.md`).

pub mod bayesian;
pub mod lasso;
pub mod linear;
pub mod pcr;
pub mod ridge;

pub use bayesian::BayesianLinear;
pub use lasso::{fit as fit_lasso, LassoModel};
pub use linear::{fit as fit_linear, LinearModel};
pub use pcr::PcrModel;
pub use ridge::{fit as fit_ridge, RidgeModel};
