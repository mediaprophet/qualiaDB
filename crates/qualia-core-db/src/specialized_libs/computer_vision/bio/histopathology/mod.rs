//! Histopathology color / stain algorithms (pure Rust, caller-buffered).
//!
//! CIE LAB (D65), optical density, Reinhard normalize, Macenko deconvolution,
//! background illumination correction, and lite non-negative unmixing.
//!
//! No Python, no OpenCV ABI. Hot paths take caller-owned buffers.

mod apply_background_correct;
mod background_intensity_sample;
mod lab_to_rgb;
mod macenko_deconvolution;
mod optical_density;
mod reinhard_normalize;
mod rgb_to_lab;
mod snmf_unmix_lite;

pub use apply_background_correct::apply_background_correct;
pub use background_intensity_sample::{background_intensity_sample, RgbBg};
pub use lab_to_rgb::lab_to_rgb;
pub use macenko_deconvolution::{macenko_deconvolution, MacenkoResult};
pub use optical_density::optical_density_rgb;
pub use reinhard_normalize::{
    reinhard_normalize, LabStats, DEFAULT_HE_TARGET_MEAN, DEFAULT_HE_TARGET_STD,
};
pub use rgb_to_lab::rgb_to_lab;
pub use snmf_unmix_lite::{snmf_unmix_lite, StainBasis};

/// Errors for histopathology kernels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoError {
    BufferTooSmall,
    DimensionMismatch,
    InvalidParameter,
    EmptyInput,
    DegenerateData,
}

impl core::fmt::Display for HistoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "output buffer too small"),
            Self::DimensionMismatch => write!(f, "dimension mismatch"),
            Self::InvalidParameter => write!(f, "invalid parameter"),
            Self::EmptyInput => write!(f, "empty input"),
            Self::DegenerateData => write!(f, "degenerate or insufficient data"),
        }
    }
}
