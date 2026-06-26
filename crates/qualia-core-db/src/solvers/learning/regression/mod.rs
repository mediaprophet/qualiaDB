//! Regression estimators (ISL ch 3, 6). Built on `linear_algebra` + `statistics`.
//!
//! - [`linear`] — multiple OLS with full inference.
//!
//! Ridge / lasso / PCR / PLS land here next (build order in `stats_plan.md`).

pub mod lasso;
pub mod linear;
pub mod ridge;

pub use lasso::{fit as fit_lasso, LassoModel};
pub use linear::{fit as fit_linear, LinearModel};
pub use ridge::{fit as fit_ridge, RidgeModel};
