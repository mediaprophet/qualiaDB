//! Errors for classical CV kernels.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CvError {
    BufferTooSmall,
    DimensionMismatch,
    InvalidParameter,
    EmptyInput,
}

impl core::fmt::Display for CvError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "output buffer too small"),
            Self::DimensionMismatch => write!(f, "dimension mismatch"),
            Self::InvalidParameter => write!(f, "invalid parameter"),
            Self::EmptyInput => write!(f, "empty input"),
        }
    }
}
