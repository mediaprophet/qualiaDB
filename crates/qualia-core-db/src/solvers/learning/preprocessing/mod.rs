//! Preprocessing for statistical learning — feature scaling and design-matrix prep.
//! Reuses `statistics::descriptive`. Train/test partitioning lives with the
//! `resampling` module (it shares the fold/shuffle machinery).

pub mod scaling;

pub use scaling::StandardScaler;
