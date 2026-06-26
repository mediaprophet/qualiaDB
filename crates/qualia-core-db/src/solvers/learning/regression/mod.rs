//! Regression estimators (ISL ch 3, 6). Built on `linear_algebra` + `statistics`.
//!
//! - [`linear`] — multiple OLS with full inference.
//!
//! Ridge / lasso / PCR / PLS land here next (build order in `stats_plan.md`).

pub mod linear;

pub use linear::{fit as fit_linear, LinearModel};
