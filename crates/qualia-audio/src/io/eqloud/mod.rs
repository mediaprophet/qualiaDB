//! Equal-loudness weighting / loader. Re-exports only (AU-IO).
//!
//! - [`apply_equal_loudness`] weights a magnitude spectrum by the A-weighting
//!   equal-loudness curve.
//! - [`perceptual_load_from_wav`] combines decode + weighting into a single
//!   perceptual-load scalar.

pub mod equal_loudness;
pub mod eqloud_loader;

pub use eqloud_loader::{perceptual_load_from_wav, BLOCK};
pub use equal_loudness::apply_equal_loudness;
