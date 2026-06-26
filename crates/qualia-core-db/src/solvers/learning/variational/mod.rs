//! Variational inference (PRML ch 10) — approximate an intractable posterior by the
//! closest factorized distribution (mean-field), via coordinate-ascent (CAVI).
//!
//! - [`gaussian`] — the canonical univariate-Gaussian mean+precision example.

pub mod gaussian;

pub use gaussian::{fit as fit_variational_gaussian, VariationalGaussian};
