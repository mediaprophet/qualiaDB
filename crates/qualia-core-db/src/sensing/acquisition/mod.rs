//! Vendor-neutral borrowed acquisition contracts; no device access or wire allocation.
//! Byte integrity is not authority, calibration validity, or a claim of scientific truth.
mod correlation;
mod events;
mod lineage;
mod metadata;
mod source;
pub mod storage;
mod tensor;
pub use crate::audio::qac::signal::{ExactTime, Rate, Scalar, Timing};
pub use correlation::*;
pub use events::*;
pub use lineage::*;
pub use metadata::*;
pub use source::*;
pub use tensor::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Budget,
    Overflow,
    Bounds,
    Shape,
    Encoding,
    Clock,
    Identity,
    Semantics,
    Nonfinite,
    Ordering,
    LossDeclaration,
    Unsupported,
}
#[cfg(test)]
mod tests;
