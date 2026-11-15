//! Stored-domain DICOM padding admission and measurement validity.
mod float;
mod integer;
mod quantitative;
use crate::medical::dicom_codecs::PixelError;
use crate::medical::dicom_dataset::DatasetError;
pub use float::FloatPadding;
pub use integer::{IntegerPadding, IntegerPaddingView, IntegerSample};
pub(super) use quantitative::map_sample;
pub use quantitative::QuantitativeSample;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaddingError {
    Dataset(DatasetError),
    Pixel(PixelError),
    Missing(u32),
    Invalid(u32),
    SourceMismatch,
    FrameMismatch,
    WidthMismatch,
    Unsupported(&'static str),
}
impl From<DatasetError> for PaddingError {
    fn from(e: DatasetError) -> Self {
        Self::Dataset(e)
    }
}
impl From<PixelError> for PaddingError {
    fn from(e: PixelError) -> Self {
        Self::Pixel(e)
    }
}
impl std::fmt::Display for PaddingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DICOM sample validity: {self:?}")
    }
}
impl std::error::Error for PaddingError {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaddingMatch {
    NotPadding,
    Padding,
    Unordered,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleValidity {
    Valid,
    Padding,
    NonFinite,
    PaddingIndeterminate,
}
#[cfg(test)]
mod tests;
