//! Survival analysis (ISL ch 11) — time-to-event modelling with right censoring.
//! Time-indexed provenance / life-record reasoning is temporal, so these are the
//! standard estimators over censored temporal evidence.
//!
//! - [`kaplan_meier`] — nonparametric survival curve `S(t)`.
//! - [`cox`] — Cox proportional-hazards regression (covariate effects on the hazard).

pub mod cox;
pub mod kaplan_meier;

pub use cox::{fit as fit_cox, CoxModel};
pub use kaplan_meier::KaplanMeier;
