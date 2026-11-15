//! Per-frame identity/geometry/quantitative metadata; no registration inference.
mod extract;
mod quantitative;
pub use extract::{DatasetIdentity, FrameError, FrameLimits, FrameRecord, FrameSet};
pub use quantitative::{LinearTransform, QuantitativeStatus, ValueUnits};
#[cfg(test)]
mod tests;
